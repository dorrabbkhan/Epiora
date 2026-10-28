// JS-facing interface for the libmdb wasm module (v0.1 Task 4).
// Produces: mdb_version(path) -> "JET3" | "JET4" | "ERROR:<msg>",
//           mdb_tables(path)  -> string[] | "ERROR:<msg>",
//           mdb_export_csv(path, table) -> CSV string | "ERROR:<msg>"
// `path` is a host filesystem path; each call stages the file into the
// wasm module's virtual FS before invoking the exported C function.
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join, basename } from 'node:path';

const here = dirname(fileURLToPath(import.meta.url));
const dist = join(here, 'dist');

let _module = null;
async function mod() {
  if (_module) return _module;
  const { default: init } = await import(join(dist, 'mdb.js'));
  _module = await init({ printErr() {} }); // keep libmdb stderr chatter out of test output
  try { _module.FS.mkdir('/work'); } catch { /* exists */ }
  return _module;
}

function stage(M, hostPath) {
  const vpath = `/work/${basename(hostPath)}`;
  M.FS.writeFile(vpath, readFileSync(hostPath));
  return vpath;
}

export async function mdb_version(hostPath) {
  const M = await mod();
  return M.ccall('mdb_version', 'string', ['string'], [stage(M, hostPath)]);
}

export async function mdb_tables(hostPath) {
  const M = await mod();
  const r = M.ccall('mdb_tables', 'string', ['string'], [stage(M, hostPath)]);
  return r.startsWith('ERROR:') ? r : JSON.parse(r);
}

export async function mdb_export_csv(hostPath, table) {
  const M = await mod();
  return M.ccall('mdb_export_csv', 'string', ['string', 'string'],
    [stage(M, hostPath), table]);
}
