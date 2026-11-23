/* epiora-mdb harness: handle table, version/corruption/password
 * sniffing and the table list. The batch encoder lives in eptb.c.
 *
 * Password detection is jackcess-style (JetFormat.java): JET3 keeps a
 * 20-byte obfuscated password field at header offset 0x42 with no mask;
 * JET4 keeps 40 bytes at 0x42 masked by (int)creation_date — the f64 at
 * header offset 0x72 — XORed byte-wise. mdbtools itself silently reads
 * a JET4 password-protected file (the pages are not encrypted), so the
 * header sniff must happen here. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "epiora_mdb.h"

#define MAX_HANDLES 64
#define HDR_READ 4096

static EMdb *handles[MAX_HANDLES];
/* Per-call error state is thread-local: handles are shared but each
 * thread's last_error/last_format must survive another thread's calls
 * (parallel tests open several databases at once). */
static _Thread_local int last_error;
static _Thread_local char last_format[16] = "unknown";

int epiora_mdb_last_error(void) { return last_error; }
void emdb_set_last_error(int e) { last_error = e; }
const char *epiora_mdb_last_format(void) { return last_format; }

EMdb *emdb_get(int h)
{
    if (h <= 0 || h > MAX_HANDLES)
        return NULL;
    return handles[h - 1];
}

/* RC4 of the header's obfuscated region (bytes 0x18.., key C7 DA 39 6B
 * — same as mdbtools file.c). Operates on a copy; page 0 in the file
 * is untouched. */
static void header_rc4(unsigned char *buf, size_t len)
{
    static const unsigned char key[4] = {0xC7, 0xDA, 0x39, 0x6B};
    unsigned char s[256];
    unsigned i, j;
    for (i = 0; i < 256; i++)
        s[i] = (unsigned char)i;
    for (i = 0, j = 0; i < 256; i++) {
        j = (j + s[i] + key[i % 4]) & 0xff;
        unsigned char t = s[i];
        s[i] = s[j];
        s[j] = t;
    }
    for (i = 0, j = 0; len--; buf++) {
        i = (i + 1) & 0xff;
        j = (j + s[i]) & 0xff;
        unsigned char t = s[i];
        s[i] = s[j];
        s[j] = t;
        *buf ^= s[(s[i] + s[j]) & 0xff];
    }
}

static int has_password(const unsigned char *hdr, int jet_version)
{
    int i;
    if (jet_version == MDB_VER_JET3) {
        for (i = 0; i < 20; i++)
            if (hdr[0x42 + i])
                return 1;
        return 0;
    }
    {
        double d;
        int32_t key;
        unsigned char mask[4];
        memcpy(&d, hdr + 0x72, 8); /* OFFSET_HEADER_DATE, LE */
        key = (int32_t)d;
        mask[0] = key & 0xff;
        mask[1] = (key >> 8) & 0xff;
        mask[2] = (key >> 16) & 0xff;
        mask[3] = (key >> 24) & 0xff;
        for (i = 0; i < 40; i++)
            if ((hdr[0x42 + i] ^ mask[i % 4]) != 0)
                return 1;
    }
    return 0;
}

/* "windows-1250".."windows-1258" and "windows-874" are the supported
 * single-byte pages (eptb.c holds the tables). */
static int code_page_ok(const char *cp)
{
    static const char *pages[] = {
        "windows-874", "windows-1250", "windows-1251", "windows-1252",
        "windows-1253", "windows-1254", "windows-1255", "windows-1256",
        "windows-1257", "windows-1258",
    };
    size_t i;
    for (i = 0; i < sizeof(pages) / sizeof(pages[0]); i++)
        if (!strcmp(cp, pages[i]))
            return 1;
    return 0;
}

static int fail(int code)
{
    last_error = code;
    return -code;
}

int epiora_mdb_open(const char *path, const char *code_page)
{
    unsigned char hdr[HDR_READ];
    size_t n;
    FILE *f;
    int jet_version, i;
    char fmt[16] = "unknown";
    const char *cp = code_page && *code_page ? code_page : "windows-1252";

    strcpy(last_format, "unknown");
    if (!cp || !code_page_ok(cp))
        return fail(EMDB_CODEPAGE);

    f = fopen(path, "rb");
    if (!f)
        return fail(EMDB_CORRUPT);
    n = fread(hdr, 1, sizeof(hdr), f);
    fclose(f);
    if (n < 152 || hdr[0] != 0 || (memcmp(hdr + 4, "Standard Jet DB", 15) &&
                                   memcmp(hdr + 4, "Standard ACE DB", 15)))
        return fail(EMDB_CORRUPT);

    jet_version = hdr[0x14];
    if (jet_version == MDB_VER_JET3)
        strcpy(fmt, "JET3");
    else if (jet_version == MDB_VER_JET4)
        strcpy(fmt, "JET4");
    else {
        strcpy(last_format, "ACE");
        return fail(EMDB_UNSUPPORTED);
    }
    strcpy(last_format, fmt);

    /* Decrypt the header's obfuscated region on a copy and apply the
     * jackcess password check. */
    header_rc4(hdr + 0x18, jet_version == MDB_VER_JET3 ? 126 : 128);
    if (has_password(hdr, jet_version))
        return fail(EMDB_PASSWORD);

    for (i = 0; i < MAX_HANDLES; i++)
        if (__sync_bool_compare_and_swap(&handles[i], NULL, (EMdb *)1))
            break;
    if (i == MAX_HANDLES)
        return fail(EMDB_NO_HANDLE);

    {
        MdbHandle *mdb = mdb_open(path, MDB_NOFLAGS);
        if (!mdb) {
            handles[i] = NULL;
            return fail(EMDB_CORRUPT);
        }
        mdb_set_bind_size(mdb, 4 * 1024 * 1024);
        /* mdb_open reads only page 0; the catalog read is what detects
         * a truncated or damaged file. */
        if (!mdb_read_catalog(mdb, MDB_TABLE)) {
            mdb_close(mdb);
            handles[i] = NULL;
            return fail(EMDB_TRUNCATED);
        }
        handles[i] = calloc(1, sizeof(EMdb));
        handles[i]->mdb = mdb;
        handles[i]->jet_version = jet_version;
        handles[i]->db_code_page = hdr[0x3c] | (hdr[0x3d] << 8);
        strcpy(handles[i]->format, fmt);
        snprintf(handles[i]->code_page, sizeof(handles[i]->code_page),
                 "%s", cp);
        last_error = EMDB_OK;
        return i + 1;
    }
}

const char *epiora_mdb_format(int h)
{
    EMdb *e = emdb_get(h);
    if (!e) {
        last_error = EMDB_NO_HANDLE;
        return NULL;
    }
    return e->format;
}

/* JSON-escape a UTF-8 name into a growable buffer. */
static char *json_escape(const char *s)
{
    size_t len = 3, i; /* quotes + NUL */
    char *out, *p;
    for (i = 0; s[i]; i++)
        len += (unsigned char)s[i] < 0x20 || s[i] == '"' || s[i] == '\\' ? 6 : 1;
    out = malloc(len);
    p = out;
    *p++ = '"';
    for (i = 0; s[i]; i++) {
        unsigned char c = (unsigned char)s[i];
        if (c < 0x20)
            p += sprintf(p, "\\u%04x", c);
        else if (c == '"' || c == '\\') {
            *p++ = '\\';
            *p++ = (char)c;
        } else
            *p++ = (char)c;
    }
    *p++ = '"';
    *p = 0;
    return out;
}

char *epiora_mdb_tables(int h)
{
    EMdb *e = emdb_get(h);
    GPtrArray *catalog;
    char *out, *p;
    size_t cap = 4096, len = 1;
    unsigned i;

    if (!e) {
        last_error = EMDB_NO_HANDLE;
        return NULL;
    }
    catalog = e->mdb->catalog;
    for (i = 0; i < catalog->len; i++) {
        MdbCatalogEntry *entry = g_ptr_array_index(catalog, i);
        if (entry->object_type == MDB_TABLE && mdb_is_user_table(entry))
            cap += strlen(entry->object_name) + 16;
    }
    out = malloc(cap);
    p = out;
    *p++ = '[';
    for (i = 0; i < catalog->len; i++) {
        MdbCatalogEntry *entry = g_ptr_array_index(catalog, i);
        char *esc;
        if (entry->object_type != MDB_TABLE || !mdb_is_user_table(entry))
            continue;
        esc = json_escape(entry->object_name);
        len = strlen(esc);
        if (p != out + 1)
            *p++ = ',';
        memcpy(p, esc, len);
        p += len;
        free(esc);
    }
    *p++ = ']';
    *p = 0;
    last_error = EMDB_OK;
    return out;
}

void epiora_mdb_close(int h)
{
    EMdb *e = emdb_get(h);
    if (!e)
        return;
    if (e->mdb)
        mdb_close(e->mdb);
    free(e);
    handles[h - 1] = NULL;
}
