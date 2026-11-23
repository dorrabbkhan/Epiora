//! `epiora-mdb`: a safe wrapper over the vendored mdbtools libmdb
//! compiled by build.rs (`c/harness.c` + `c/eptb.c` + `c/codepages.c`).
//! Tests and tooling only — the shipped product module is the wasm
//! build in `app/public/mdb` produced by `build-wasm.sh` from the same
//! C sources (spec §4, v1.1 Task 5).
//!
//! No input can panic this crate: every failure surfaces as an
//! [`MdbError`] whose `code()` is stable.

use std::ffi::{c_char, c_void, CStr, CString};
use std::fmt;
use std::path::Path;

mod ffi {
    use super::*;
    extern "C" {
        pub fn epiora_mdb_open(path: *const c_char, code_page: *const c_char) -> i32;
        pub fn epiora_mdb_last_error() -> i32;
        pub fn epiora_mdb_last_format() -> *const c_char;
        pub fn epiora_mdb_format(h: i32) -> *const c_char;
        pub fn epiora_mdb_tables(h: i32) -> *mut c_char;
        pub fn epiora_mdb_batch(
            h: i32,
            table: *const c_char,
            offset: u32,
            limit: u32,
            out_len: *mut u32,
        ) -> *mut u8;
        pub fn epiora_mdb_close(h: i32);
        pub fn free(p: *mut c_void);
    }
}

/// The Jet/ACE format detected on the database header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MdbFormat {
    /// JET3 (Access 95/97-era `.mdb`).
    Jet3,
    /// JET4 (Access 2000+ `.mdb`, the Epi Info 7 format).
    Jet4,
    /// `.accdb` — rejected at open (`mdb_unsupported_version`).
    Ace,
}

impl MdbFormat {
    fn parse(s: &CStr) -> Self {
        match s.to_bytes() {
            b"JET3" => Self::Jet3,
            b"JET4" => Self::Jet4,
            _ => Self::Ace,
        }
    }
}

/// Every failure of the reader, with a stable `code()`.
#[derive(Debug)]
pub enum MdbError {
    /// Not a Jet/ACE database or unreadable file.
    Corrupt { path: String },
    /// Valid header but the catalog cannot be read — truncated file.
    Truncated { format: MdbFormat },
    /// The header's password field is set.
    PasswordProtected { format: MdbFormat },
    /// `.accdb`/ACE or an unknown Jet version.
    UnsupportedVersion { format: MdbFormat },
    /// The table name is not in the catalog.
    TableNotFound { table: String },
    /// An unsupported code page was requested.
    CodePageUnsupported { code_page: String },
    /// The C encoder failed while producing a batch.
    BatchEncode,
    /// `mdb_tables` returned invalid JSON.
    TablesJson,
}

impl MdbError {
    /// The stable machine-readable error code.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Corrupt { .. } => "mdb_corrupt",
            Self::Truncated { .. } => "mdb_truncated",
            Self::PasswordProtected { .. } => "mdb_password_protected",
            Self::UnsupportedVersion { .. } => "mdb_unsupported_version",
            Self::TableNotFound { .. } => "mdb_no_table",
            Self::CodePageUnsupported { .. } => "mdb_code_page_unsupported",
            Self::BatchEncode => "mdb_encode",
            Self::TablesJson => "mdb_tables_json",
        }
    }
}

impl fmt::Display for MdbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Corrupt { path } => write!(f, "not a readable Jet database: {path}"),
            Self::Truncated { format } => write!(f, "truncated {format:?} database"),
            Self::PasswordProtected { format } => {
                write!(f, "password-protected {format:?} database")
            }
            Self::UnsupportedVersion { format } => {
                write!(f, "unsupported database version: {format:?}")
            }
            Self::TableNotFound { table } => write!(f, "no such table: {table}"),
            Self::CodePageUnsupported { code_page } => {
                write!(f, "unsupported code page: {code_page}")
            }
            Self::BatchEncode => write!(f, "batch encode failed"),
            Self::TablesJson => write!(f, "table list JSON invalid"),
        }
    }
}

impl std::error::Error for MdbError {}

/// An open database. Dropping closes the handle.
///
/// libmdb and the harness's handle table are not thread-safe, so
/// `Mdb` is deliberately neither `Send` nor `Sync`.
#[derive(Debug)]
pub struct Mdb {
    handle: i32,
    _not_send: std::marker::PhantomData<*const ()>,
}

impl Mdb {
    /// Opens a database decoding JET3 text with the default
    /// `windows-1252` code page.
    pub fn open(path: &Path) -> Result<Self, MdbError> {
        Self::open_with_code_page(path, "windows-1252")
    }

    /// Opens a database, decoding JET3 text with the given single-byte
    /// code page (`windows-874`, `windows-1250`..`windows-1258`).
    pub fn open_with_code_page(path: &Path, code_page: &str) -> Result<Self, MdbError> {
        let cpath =
            CString::new(path.as_os_str().to_string_lossy().into_owned()).map_err(|_| {
                MdbError::Corrupt {
                    path: path.display().to_string(),
                }
            })?;
        let ccp = CString::new(code_page).map_err(|_| MdbError::CodePageUnsupported {
            code_page: code_page.to_string(),
        })?;
        let h = unsafe { ffi::epiora_mdb_open(cpath.as_ptr(), ccp.as_ptr()) };
        if h > 0 {
            return Ok(Self {
                handle: h,
                _not_send: std::marker::PhantomData,
            });
        }
        let format = MdbFormat::parse(unsafe { CStr::from_ptr(ffi::epiora_mdb_last_format()) });
        Err(match -h {
            2 => MdbError::Truncated { format },
            3 => MdbError::PasswordProtected { format },
            4 => MdbError::UnsupportedVersion { format },
            8 => MdbError::CodePageUnsupported {
                code_page: code_page.to_string(),
            },
            _ => MdbError::Corrupt {
                path: path.display().to_string(),
            },
        })
    }

    /// The database's detected format.
    pub fn format(&self) -> MdbFormat {
        MdbFormat::parse(unsafe { CStr::from_ptr(ffi::epiora_mdb_format(self.handle)) })
    }

    /// The user tables in catalog order (system objects filtered).
    pub fn tables(&self) -> Result<Vec<String>, MdbError> {
        let raw = unsafe { ffi::epiora_mdb_tables(self.handle) };
        if raw.is_null() {
            return Err(MdbError::TablesJson);
        }
        let json = unsafe { CStr::from_ptr(raw) }
            .to_string_lossy()
            .into_owned();
        unsafe { ffi::free(raw.cast()) };
        serde_json::from_str(&json).map_err(|_| MdbError::TablesJson)
    }

    /// One EPTB v1 batch for `table` rows `[offset, offset+limit)`.
    pub fn batch(&self, table: &str, offset: u32, limit: u32) -> Result<Vec<u8>, MdbError> {
        let cname = CString::new(table).map_err(|_| MdbError::TableNotFound {
            table: table.to_string(),
        })?;
        let mut out_len = 0u32;
        let raw = unsafe {
            ffi::epiora_mdb_batch(self.handle, cname.as_ptr(), offset, limit, &mut out_len)
        };
        if raw.is_null() {
            let code = unsafe { ffi::epiora_mdb_last_error() };
            return Err(match code {
                5 => MdbError::TableNotFound {
                    table: table.to_string(),
                },
                8 => MdbError::CodePageUnsupported {
                    code_page: String::new(),
                },
                _ => MdbError::BatchEncode,
            });
        }
        let bytes = unsafe { std::slice::from_raw_parts(raw, out_len as usize) }.to_vec();
        unsafe { ffi::free(raw.cast()) };
        Ok(bytes)
    }
}

impl Drop for Mdb {
    fn drop(&mut self) {
        unsafe { ffi::epiora_mdb_close(self.handle) };
    }
}

/// Fuzz entry point: write `data` to a temp file, open it and read
/// every table's first batch. Exposed under `feature = "fuzzing"` for
/// the `mdb_any` cargo-fuzz target.
#[cfg(feature = "fuzzing")]
pub fn mdb_any(data: &[u8]) {
    let path = std::env::temp_dir().join(format!("epiora-mdb-fuzz-{}.mdb", std::process::id()));
    if std::fs::write(&path, data).is_err() {
        return;
    }
    if let Ok(mdb) = Mdb::open(&path) {
        let _ = mdb.format();
        if let Ok(tables) = mdb.tables() {
            for t in &tables {
                let _ = mdb.batch(t, 0, 64);
            }
        }
    }
    let _ = std::fs::remove_file(&path);
}

#[cfg(test)]
mod tests {
    use super::*;

    extern "C" {
        /// C test hook (native build only — absent from the wasm
        /// export list): escape one catalog name exactly as
        /// `epiora_mdb_tables` does.
        fn epiora_mdb_test_json_escape(s: *const c_char) -> *mut c_char;
    }

    /// A catalog name at the `MDB_MAX_OBJ_NAME` bound made only of
    /// control bytes is the worst case for the table-list buffer:
    /// every byte escapes to the 6-byte `\u00XX` form. The JSON must
    /// parse and round-trip to the original name.
    #[test]
    fn json_escape_round_trips_all_control_byte_name() {
        let name = "\u{1}".repeat(255);
        let cname = CString::new(name.clone()).unwrap();
        let raw = unsafe { epiora_mdb_test_json_escape(cname.as_ptr()) };
        assert!(!raw.is_null());
        let json = unsafe { CStr::from_ptr(raw) }.to_str().unwrap().to_owned();
        unsafe { ffi::free(raw.cast()) };
        let parsed: String = serde_json::from_str(&json).expect("escaped JSON parses");
        assert_eq!(parsed, name, "round-trips to the original name");
    }
}
