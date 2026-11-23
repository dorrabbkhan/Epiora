//! Compiles the vendored mdbtools 1.0.1 libmdb sources plus the EPTB
//! harness natively with `cc`, mirroring the spike's emscripten build
//! (`--disable-glib --disable-iconv`): `mdbfakeglib.h` stands in for
//! glib and `mdbtools.h` is generated here from the template with
//! iconv/xlocale disabled, because autoconf is not a build dependency.

use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let vendor = root.join("vendor/mdbtools-1.0.1");
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());

    // mdbtools.h is normally produced by configure; generate the
    // no-glib/no-iconv variant the wasm build uses.
    let template = fs::read_to_string(vendor.join("include/mdbtools.h.in")).unwrap();
    let header = template
        .replace("@HAVE_ICONV_H@", "0")
        .replace("@HAVE_XLOCALE_H@", "0")
        .replace("@GLIB_INCLUDE_HEADER@", "mdbfakeglib.h")
        .replace("@LOCALE_T@", "locale_t");
    let gen_include = out.join("include");
    fs::create_dir_all(&gen_include).unwrap();
    fs::write(gen_include.join("mdbtools.h"), header).unwrap();

    let mut sources: Vec<PathBuf> = fs::read_dir(vendor.join("src/libmdb"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "c"))
        .collect();
    sources.sort();
    sources.push(root.join("c/harness.c"));
    sources.push(root.join("c/eptb.c"));
    sources.push(root.join("c/codepages.c"));

    cc::Build::new()
        .files(&sources)
        .include(&gen_include)
        .include(vendor.join("include"))
        .include(root.join("c"))
        // TLS is a fake-glib macro for __thread; _GNU_SOURCE and the
        // HAVE_* set mirror the configure DEFS for a glibc build.
        .define("TLS", "_Thread_local")
        .define("_GNU_SOURCE", "1")
        .define("HAVE_FMEMOPEN", "1")
        .define("HAVE_VASPRINTF", "1")
        .define("HAVE_GMTIME_R", "1")
        .define("HAVE_STRPTIME", "1")
        .define("HAVE_ATTRIBUTE_ALIAS", "1")
        .define("HAVE_STRNCASECMP", "1")
        .warnings(false)
        .compile("epiora_mdb");

    println!("cargo:rerun-if-changed=c");
    println!("cargo:rerun-if-changed=vendor");
    println!("cargo:rerun-if-changed=build.rs");
}
