/*
 * harness.c — Emscripten glue exposing libmdb (mdbtools 1.0.1) to JavaScript.
 *
 * Exported via EXPORTED_FUNCTIONS in build.sh:
 *   char *mdb_version(const char *path)
 *       -> "JET3" | "JET4" | "ACE12"|"ACE14"|"ACE15"|"ACE16"|"ACE18"
 *          | "ERROR:<message>"   (the plan's interface lists only JET3/JET4;
 *          ACE files are newer Access formats mdb-ver also names)
 *   char *mdb_tables(const char *path)
 *       -> JSON array of user-table names | "ERROR:<message>"
 *   char *mdb_export_csv(const char *path, const char *table)
 *       -> CSV text byte-identical to `mdb-export` defaults | "ERROR:<message>"
 *
 * Every returned string is malloc'd; callers may _free it (also exported).
 * A truncated or corrupt file must yield an "ERROR:" string that names the
 * detected format — never a crash and never a silent empty result.
 */
#include <emscripten.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "mdbtools.h"

#define EXPORT_BIND_SIZE 200000   /* same buffer size as src/util/mdb-export.c */

static char *vformat(const char *fmt, const char *a, const char *b)
{
	size_t n = strlen(fmt) + (a ? strlen(a) : 0) + (b ? strlen(b) : 0) + 64;
	char *s = malloc(n);
	snprintf(s, n, fmt, a ? a : "", b ? b : "");
	return s;
}

static char *vformat_ui(const char *fmt, unsigned int u, const char *a)
{
	char nbuf[16];
	snprintf(nbuf, sizeof(nbuf), "%u", u);
	return vformat(fmt, nbuf, a);
}

static const char *jet_name(int v)
{
	switch (v) {
	case MDB_VER_JET3:       return "JET3";
	case MDB_VER_JET4:       return "JET4";
	case MDB_VER_ACCDB_2007: return "ACE12";
	case MDB_VER_ACCDB_2010: return "ACE14";
	case MDB_VER_ACCDB_2013: return "ACE15";
	case MDB_VER_ACCDB_2016: return "ACE16";
	case MDB_VER_ACCDB_2019: return "ACE18";
	default:                 return NULL;
	}
}

/* Sniff the file header directly so a failure deeper in can still name the
 * format.  Returns a static/string-literal name or NULL when unrecognised. */
static const char *sniff_jet(const char *path)
{
	static char unknown[48];
	unsigned char pg[32];
	size_t n;
	FILE *f = fopen(path, "rb");
	if (!f) return NULL;
	n = fread(pg, 1, sizeof(pg), f);
	fclose(f);
	if (n <= 0x14) return NULL;
	if (memcmp(pg + 4, "Standard Jet DB", 15) != 0 &&
	    memcmp(pg + 4, "Standard ACE DB", 15) != 0)
		return NULL;
	{
		const char *name = jet_name(pg[0x14]);
		if (name) return name;
		snprintf(unknown, sizeof(unknown), "unknown Jet version 0x%02x", pg[0x14]);
		return unknown;
	}
}

/* Open the database AND prove the reader can handle it by reading the table
 * catalog (mdb_open itself only reads page 0, so a file truncated after the
 * header still opens).  On failure returns NULL and sets *err. */
static MdbHandle *open_db(const char *path, char **err)
{
	MdbHandle *mdb = mdb_open(path, MDB_NOFLAGS);
	if (!mdb) {
		const char *sniff = sniff_jet(path);
		*err = sniff
			? vformat("ERROR:%s file unreadable or corrupt", sniff, NULL)
			: vformat("ERROR:not a Jet/ACE database or unreadable file: %s",
				  path, NULL);
		return NULL;
	}
	if (!mdb_read_catalog(mdb, MDB_TABLE)) {
		const char *name = jet_name(mdb->f->jet_version);
		*err = vformat(
			"ERROR:%s file truncated or corrupt: cannot read table catalog",
			name ? name : "unknown-format", NULL);
		mdb_close(mdb);
		return NULL;
	}
	return mdb;
}

EMSCRIPTEN_KEEPALIVE
char *mdb_version(const char *path)
{
	char *err = NULL, *res;
	MdbHandle *mdb = open_db(path, &err);
	const char *name;
	if (!mdb) return err;
	/* mdb_open rejects unknown version bytes, so jet_name cannot fail here */
	name = jet_name(mdb->f->jet_version);
	res = strdup(name ? name : "ERROR:unknown Jet version");
	mdb_close(mdb);
	return res;
}

EMSCRIPTEN_KEEPALIVE
char *mdb_tables(const char *path)
{
	char *err = NULL;
	MdbHandle *mdb = open_db(path, &err);
	size_t cap, len = 0;
	char *out;
	unsigned int i;
	if (!mdb) return err;

	cap = 4096;
	out = malloc(cap);
	out[len++] = '[';
	for (i = 0; i < mdb->catalog->len; i++) {
		MdbCatalogEntry *e = g_ptr_array_index(mdb->catalog, i);
		const char *p;
		if (e->object_type != MDB_TABLE) continue;
		if (len && out[len - 1] != '[') out[len++] = ',';
		out[len++] = '"';
		for (p = e->object_name; *p; p++) {
			char esc[3] = {0, 0, 0};
			const char *chunk = p;
			size_t clen = 1;
			if (*p == '"' || *p == '\\') {
				esc[0] = '\\'; esc[1] = *p;
				chunk = esc; clen = 2;
			}
			while (len + clen + 8 > cap) {
				cap *= 2;
				out = realloc(out, cap);
			}
			memcpy(out + len, chunk, clen);
			len += clen;
		}
		out[len++] = '"';
	}
	out[len++] = ']';
	out[len] = '\0';
	mdb_close(mdb);
	return out;
}

EMSCRIPTEN_KEEPALIVE
char *mdb_export_csv(const char *path, const char *table_name)
{
	char *err = NULL;
	MdbHandle *mdb = open_db(path, &err);
	MdbTableDef *table;
	FILE *out;
	char **vals;
	int *lens;
	long size;
	char *csv;
	unsigned int i;
	if (!mdb) return err;

	table = mdb_read_table_by_name(mdb, (char *)table_name, MDB_TABLE);
	if (!table) {
		mdb_close(mdb);
		return vformat("ERROR:table '%s' not found in %s",
			       table_name, path);
	}
	mdb_read_columns(table);
	mdb_rewind_table(table);

	out = tmpfile();
	if (!out) {
		mdb_free_tabledef(table);
		mdb_close(mdb);
		return vformat("ERROR:cannot create temp stream exporting %s",
			       table_name, NULL);
	}

	vals = malloc(table->num_cols * sizeof(char *));
	lens = malloc(table->num_cols * sizeof(int));
	for (i = 0; i < table->num_cols; i++) {
		vals[i] = calloc(1, EXPORT_BIND_SIZE);
		lens[i] = 0;
		if (mdb_bind_column(table, i + 1, vals[i], &lens[i]) == -1) {
			for (; (int)i >= 0; i--) free(vals[i]);
			free(vals); free(lens);
			fclose(out);
			mdb_free_tabledef(table);
			mdb_close(mdb);
			return vformat_ui("ERROR:failed to bind column %u of %s",
					  i + 1, table_name);
		}
	}

	/* header row: bare column names, comma-separated (as mdb-export) */
	for (i = 0; i < table->num_cols; i++) {
		MdbColumn *col = g_ptr_array_index(table->columns, i);
		if (i) fputc(',', out);
		fputs(col->name, out);
	}
	fputc('\n', out);

	while (mdb_fetch_row(table)) {
		for (i = 0; i < table->num_cols; i++) {
			MdbColumn *col = g_ptr_array_index(table->columns, i);
			if (i) fputc(',', out);
			if (!lens[i]) continue;            /* NULL -> empty */
			if (col->col_type == MDB_OLE) {
				size_t olen = 0;
				void *ov = mdb_ole_read_full(mdb, col, &olen);
				mdb_print_col(out, ov, 1, MDB_OLE, olen,
					      "\"", NULL, MDB_BINEXPORT_RAW);
				free(ov);
			} else {
				mdb_print_col(out, vals[i], 1, col->col_type,
					      lens[i], "\"", NULL,
					      MDB_BINEXPORT_RAW);
			}
		}
		fputc('\n', out);
	}

	fflush(out);
	fseek(out, 0, SEEK_END);
	size = ftell(out);
	rewind(out);
	csv = malloc(size + 1);
	if (fread(csv, 1, size, out) != (size_t)size) {
		free(csv);
		csv = vformat("ERROR:temp stream read failed exporting %s",
			      table_name, NULL);
	}
	else csv[size] = '\0';
	fclose(out);

	for (i = 0; i < table->num_cols; i++) free(vals[i]);
	free(vals);
	free(lens);
	mdb_free_tabledef(table);
	mdb_close(mdb);
	return csv;
}
