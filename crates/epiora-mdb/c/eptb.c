/* EPTB v1 batch encoder (v1.1 plan Task 5). Reads one table chunk with
 * libmdb's row iterator and serialises it column-major:
 *
 *   "EPTB" u16 v1 | u32 row_count | u16 col_count
 *   u16 ext_len | ext { u8 tag, u16 len, bytes }*
 *        tag1 = table name, tag2 = code page, tag3 = u32 decode errors
 *   per column: u32 name_len + name | u8 source_type | u8 kind
 *        kinds 0-4: missing bitmap ceil(row_count/8) then present-only
 *        values (i64 / f64 / NUL-terminated-string blob + offsets /
 *        u8 / Access-days f64); kind 5 (binary-skipped): nothing.
 *
 * JET3 text: mdbtools was built without iconv, so bound strings are
 * latin1-mapped UTF-8 when the file's declared page is 1252 and raw
 * bytes otherwise. Both recover to the original byte stream, which is
 * then decoded with the caller's single-byte page; bytes undefined in
 * that page become U+FFFD and count in decode_errors. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "epiora_mdb.h"

const uint16_t *epiora_cp_table(const char *name);
void emdb_set_last_error(int e);

/* ---------------------------------------------------------------- */

typedef struct {
    unsigned char *b;
    size_t len, cap;
    int failed;
} Buf;

static void put(Buf *w, const void *p, size_t n)
{
    if (w->failed)
        return;
    if (w->len + n > w->cap) {
        size_t nc = w->cap ? w->cap * 2 : 4096;
        unsigned char *nb;
        while (nc < w->len + n)
            nc *= 2;
        nb = realloc(w->b, nc);
        if (!nb) {
            w->failed = 1;
            return;
        }
        w->b = nb;
        w->cap = nc;
    }
    memcpy(w->b + w->len, p, n);
    w->len += n;
}
static void put_u8(Buf *w, unsigned v) { unsigned char c = v; put(w, &c, 1); }
static void put_u16(Buf *w, unsigned v)
{
    unsigned char c[2] = {v & 0xff, (v >> 8) & 0xff};
    put(w, c, 2);
}
static void put_u32(Buf *w, uint32_t v)
{
    unsigned char c[4] = {v & 0xff, (v >> 8) & 0xff,
                          (v >> 16) & 0xff, (v >> 24) & 0xff};
    put(w, c, 4);
}
static void put_f64(Buf *w, double d) { put(w, &d, 8); }
static void put_i64(Buf *w, int64_t v) { put(w, &v, 8); }
static void put_utf8_cp(Buf *w, unsigned cp)
{
    unsigned char c[4];
    size_t n;
    if (cp < 0x80) {
        c[0] = cp;
        n = 1;
    } else if (cp < 0x800) {
        c[0] = 0xc0 | (cp >> 6);
        c[1] = 0x80 | (cp & 0x3f);
        n = 2;
    } else {
        c[0] = 0xe0 | (cp >> 12);
        c[1] = 0x80 | ((cp >> 6) & 0x3f);
        c[2] = 0x80 | (cp & 0x3f);
        n = 3;
    }
    put(w, c, n);
}

/* ---------------------------------------------------------------- */

typedef struct {
    char *name;
    unsigned char src_type, kind;
    Buf vals;             /* kinds 0-4: packed values; kind 2: blob */
    Buf offs;             /* kind 2: u32 blob offsets, present-only */
    Buf miss;             /* missing bitmap, grown a byte per 8 rows */
    MdbColumn *col;
} ColAcc;

static int col_kind(int t)
{
    switch (t) {
    case MDB_BOOL:
        return 3;
    case MDB_BYTE:
    case MDB_INT:
    case MDB_LONGINT:
        return 0;
    case MDB_MONEY:
    case MDB_FLOAT:
    case MDB_DOUBLE:
        return 1;
    case MDB_DATETIME:
        return 4;
    case MDB_TEXT:
    case MDB_MEMO:
    case MDB_REPID:
    case MDB_NUMERIC:
        return 2;
    default:
        return 5;
    }
}

/* Append the JET3 byte stream (un-latin1-mapped or raw) decoded with
 * `table` to `dst`, NUL-terminated. */
static void jet3_text(Buf *dst, const unsigned char *s, size_t len,
                      int db_cp_is_1252, const uint16_t *table,
                      uint32_t *errors)
{
    size_t i = 0;
    while (i < len) {
        unsigned char b;
        if (db_cp_is_1252 && s[i] >= 0xc0 && s[i] < 0xc4 && i + 1 < len &&
            (s[i + 1] & 0xc0) == 0x80) {
            /* mdbtools' latin1 pass encoded bytes 0x80-0xFF as two-byte
             * UTF-8; undo it to recover the original byte. */
            b = ((s[i] & 0x03) << 6) | (s[i + 1] & 0x3f);
            i += 2;
        } else {
            b = s[i++];
        }
        if (b < 0x80)
            put_u8(dst, b);
        else {
            unsigned cp = table[b - 0x80];
            if (!cp) {
                cp = 0xfffd;
                (*errors)++;
            }
            put_utf8_cp(dst, cp);
        }
    }
    put_u8(dst, 0);
}

/* JET4 text is real UTF-8; copy it, replacing invalid bytes with
 * U+FFFD so the batch is always valid UTF-8. */
static void jet4_text(Buf *dst, const unsigned char *s, size_t len,
                      uint32_t *errors)
{
    size_t i = 0;
    while (i < len) {
        unsigned c = s[i];
        size_t need = c < 0x80 ? 0 : c < 0xe0 ? 1 : c < 0xf0 ? 2 : 3;
        size_t k;
        if (!need && c < 0x80) {
            put_u8(dst, c);
            i++;
            continue;
        }
        for (k = 1; k <= need; k++)
            if (i + k >= len || (s[i + k] & 0xc0) != 0x80)
                break;
        if (k <= need || c >= 0xf8) {
            put_utf8_cp(dst, 0xfffd);
            (*errors)++;
            i++;
        } else {
            put(dst, s + i, need + 1);
            i += need + 1;
        }
    }
    put_u8(dst, 0);
}

/* ---------------------------------------------------------------- */

static void acc_free(ColAcc *a, int n)
{
    int i;
    for (i = 0; i < n; i++) {
        free(a[i].name);
        free(a[i].vals.b);
        free(a[i].offs.b);
        free(a[i].miss.b);
    }
    free(a);
}

unsigned char *epiora_mdb_batch(int h, const char *table_name,
                                uint32_t offset, uint32_t limit,
                                uint32_t *out_len)
{
    EMdb *e = emdb_get(h);
    MdbTableDef *table;
    ColAcc *accs = NULL;
    unsigned n_cols, i;
    uint32_t row_count = 0, errors = 0, skipped = 0;
    const uint16_t *cp_table;
    Buf out = {0}, ext = {0};
    unsigned char *result = NULL;

    *out_len = 0;
    if (!e) {
        emdb_set_last_error(EMDB_NO_HANDLE);
        return NULL;
    }
    cp_table = epiora_cp_table(e->code_page);
    if (!cp_table) {
        emdb_set_last_error(EMDB_CODEPAGE);
        return NULL;
    }
    table = mdb_read_table_by_name(e->mdb, (char *)table_name, MDB_TABLE);
    if (!table || !mdb_read_columns(table)) {
        if (table)
            mdb_free_tabledef(table);
        emdb_set_last_error(EMDB_NO_TABLE);
        return NULL;
    }
    mdb_rewind_table(table);
    n_cols = table->num_cols;
    accs = calloc(n_cols, sizeof(ColAcc));
    if (!accs)
        goto encode_fail;
    for (i = 0; i < n_cols; i++) {
        MdbColumn *col = g_ptr_array_index(table->columns, i);
        accs[i].col = col;
        accs[i].src_type = col->col_type;
        accs[i].kind = col_kind(col->col_type);
        accs[i].name = strdup(col->name);
        if (!accs[i].name ||
            !mdb_bind_column(table, i + 1,
                             malloc(e->mdb->bind_size), NULL))
            goto encode_fail;
    }

    while (mdb_fetch_row(table)) {
        unsigned r;
        if (skipped < offset) {
            skipped++;
            continue;
        }
        if (row_count >= limit)
            break;
        r = row_count++;
        for (i = 0; i < n_cols; i++) {
            ColAcc *a = &accs[i];
            MdbColumn *col = a->col;
            const char *v;
            if (a->kind == 5)
                continue;
            /* For MDB_BOOL the JET null bit is the value itself
             * (isnull -> "0"), never missingness — read it first. */
            if (a->kind == 3) {
                v = col->bind_ptr ? col->bind_ptr : "";
                put_u8(&a->vals, v[0] == '1');
                continue;
            }
            if (col->is_null) {
                while (a->miss.len <= r / 8)
                    put_u8(&a->miss, 0);
                a->miss.b[r / 8] |= 1 << (r % 8);
                continue;
            }
            v = col->bind_ptr ? col->bind_ptr : "";
            switch (a->kind) {
            case 0: {
                char *end;
                long long x = strtoll(v, &end, 10);
                if (end == v)
                    errors++;
                put_i64(&a->vals, (int64_t)x);
                break;
            }
            case 1: {
                char *end;
                double x = strtod(v, &end);
                if (end == v)
                    errors++;
                put_f64(&a->vals, x);
                break;
            }
            case 4:
                put_f64(&a->vals,
                        mdb_get_double(e->mdb->pg_buf,
                                       col->cur_value_start));
                break;
            default: { /* 2: text */
                put_u32(&a->offs, a->vals.len);
                if (e->jet_version == MDB_VER_JET3)
                    jet3_text(&a->vals, (const unsigned char *)v,
                              strlen(v), e->db_code_page == 1252,
                              cp_table, &errors);
                else
                    jet4_text(&a->vals, (const unsigned char *)v,
                              strlen(v), &errors);
                break;
            }
            }
        }
    }

    /* ---- serialise ---- */
    put(&ext, "\x01", 1);
    put_u16(&ext, strlen(table_name));
    put(&ext, table_name, strlen(table_name));
    if (e->jet_version == MDB_VER_JET3) {
        /* tag 2 records the code page JET3 text was assumed to be. */
        put_u8(&ext, 2);
        put_u16(&ext, strlen(e->code_page));
        put(&ext, e->code_page, strlen(e->code_page));
    }
    put_u8(&ext, 3);
    put_u16(&ext, 4);
    put_u32(&ext, errors);

    put(&out, "EPTB", 4);
    put_u16(&out, 1);
    put_u32(&out, row_count);
    put_u16(&out, n_cols);
    put_u16(&out, ext.len);
    put(&out, ext.b, ext.len);
    for (i = 0; i < n_cols; i++) {
        ColAcc *a = &accs[i];
        put_u32(&out, strlen(a->name));
        put(&out, a->name, strlen(a->name));
        put_u8(&out, a->src_type);
        put_u8(&out, a->kind);
        if (a->kind == 5)
            continue;
        while (a->miss.len < (row_count + 7) / 8)
            put_u8(&a->miss, 0);
        put(&out, a->miss.b, a->miss.len);
        if (a->kind == 2) {
            put_u32(&out, a->vals.len);
            put(&out, a->vals.b, a->vals.len);
            put(&out, a->offs.b, a->offs.len);
        } else {
            put(&out, a->vals.b, a->vals.len);
        }
    }
    if (out.failed || ext.failed)
        goto encode_fail;
    result = out.b;
    *out_len = out.len;

encode_fail:
    if (!result) {
        emdb_set_last_error(EMDB_ENCODE);
        free(out.b);
    }
    free(ext.b);
    if (accs) {
        for (i = 0; i < n_cols; i++)
            free(accs[i].col ? accs[i].col->bind_ptr : NULL);
        acc_free(accs, n_cols);
    }
    mdb_free_tabledef(table);
    return result;
}
