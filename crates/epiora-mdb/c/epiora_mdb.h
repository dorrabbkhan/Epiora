/* epiora-mdb harness interface: the narrow C surface shared by the
 * native build (build.rs/cc -> src/lib.rs) and the emscripten build
 * (build-wasm.sh -> app/public/mdb). Error codes are stable; the
 * Rust and TS layers translate them into typed errors. */
#ifndef EPIORA_MDB_H
#define EPIORA_MDB_H

#include <stdint.h>

#define EMDB_OK 0
#define EMDB_CORRUPT 1          /* mdb_corrupt: not a Jet database */
#define EMDB_TRUNCATED 2        /* mdb_truncated: header ok, catalog unreadable */
#define EMDB_PASSWORD 3         /* mdb_password_protected */
#define EMDB_UNSUPPORTED 4      /* mdb_unsupported_version: ACE/unknown version */
#define EMDB_NO_TABLE 5         /* table name not in the catalog */
#define EMDB_NO_HANDLE 6        /* bad handle */
#define EMDB_ENCODE 7           /* batch read/encode failure */
#define EMDB_CODEPAGE 8         /* unsupported code page requested */

/* Opens a database. Returns >0 handle on success, -(error code) on
 * failure; epiora_mdb_last_format() names the detected format. */
int epiora_mdb_open(const char *path, const char *code_page);
/* Last error code for calls that return NULL/-1 through the generic
 * paths (tables/batch/format). */
int epiora_mdb_last_error(void);
/* "JET3" | "JET4" | "ACE" — always valid after an open attempt. */
const char *epiora_mdb_last_format(void);
/* NULL on bad handle. */
const char *epiora_mdb_format(int h);
/* malloc'd JSON array of user-table names; NULL on error. */
char *epiora_mdb_tables(int h);
/* One EPTB v1 batch: table rows [offset, offset+limit). malloc'd;
 * *out_len is the byte count. NULL on error (epiora_mdb_last_error). */
unsigned char *epiora_mdb_batch(int h, const char *table,
                                uint32_t offset, uint32_t limit,
                                uint32_t *out_len);
void epiora_mdb_close(int h);

/* internal: shared between harness.c and eptb.c */
#include "mdbtools.h"
typedef struct {
    MdbHandle *mdb;
    int jet_version;
    char format[16];       /* "JET3" | "JET4" */
    char code_page[24];    /* effective single-byte page, e.g. "windows-1252" */
    uint16_t db_code_page; /* the file's own declared code page */
} EMdb;

EMdb *emdb_get(int h);
void emdb_set_last_error(int e);

#endif
