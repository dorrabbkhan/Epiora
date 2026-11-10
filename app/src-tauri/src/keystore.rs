// The desktop database key's source (v0.3 plan Task 14, spec §7.3):
// 32 random bytes minted on first launch and kept in the OS keychain
// as 64 lowercase hex under service `org.epiora.app`, user
// `database-key-v1`. The key never crosses IPC and never appears in
// logs or error messages — every `KeyError` is written without it.

/// Where the database key comes from. `OsKeychain` is the app's
/// implementation; the trait exists so tests can drive both the
/// success and the failure paths (`MemoryKeychain`).
pub trait KeySource {
    /// Returns the stored key, minting and persisting a fresh one on
    /// first launch.
    fn get_or_create(&self) -> Result<[u8; 32], KeyError>;
}

/// `code()` is the wire `code` the relay reports — the same strings
/// Task 8's `StoreError` uses. `Display` never carries key material.
#[derive(Debug)]
pub enum KeyError {
    /// The OS keychain cannot be reached; there is no plaintext
    /// fallback (spec §7.3).
    Unavailable,
    /// The stored entry is not 64 lowercase hex — the file can never
    /// be opened with it.
    Malformed,
}

impl KeyError {
    pub fn code(&self) -> &'static str {
        match self {
            KeyError::Unavailable => "keychain_unavailable",
            KeyError::Malformed => "key_invalid",
        }
    }
}

impl std::fmt::Display for KeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyError::Unavailable => write!(f, "the OS keychain is not available"),
            KeyError::Malformed => write!(f, "the stored database key is not valid"),
        }
    }
}

impl std::error::Error for KeyError {}

const SERVICE: &str = "org.epiora.app";
const USER: &str = "database-key-v1";

fn mint() -> Result<[u8; 32], KeyError> {
    let mut key = [0u8; 32];
    getrandom::fill(&mut key).map_err(|_| KeyError::Unavailable)?;
    Ok(key)
}

fn to_hex(key: &[u8; 32]) -> String {
    let mut s = String::with_capacity(64);
    for b in key {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

fn parse_key(stored: &str) -> Result<[u8; 32], KeyError> {
    let bytes = stored.as_bytes();
    if bytes.len() != 64 {
        return Err(KeyError::Malformed);
    }
    let nibble = |c: u8| -> Result<u8, KeyError> {
        match c {
            b'0'..=b'9' => Ok(c - b'0'),
            b'a'..=b'f' => Ok(c - b'a' + 10),
            _ => Err(KeyError::Malformed),
        }
    };
    let mut key = [0u8; 32];
    for (i, pair) in bytes.as_chunks::<2>().0.iter().enumerate() {
        key[i] = (nibble(pair[0])? << 4) | nibble(pair[1])?;
    }
    Ok(key)
}

/// The flow both `KeySource`s share: read the entry; on a miss mint a
/// fresh key with `getrandom` and persist it as lowercase hex.
fn get_or_create_entry(
    read: impl Fn() -> Result<Option<String>, KeyError>,
    write: impl Fn(&str) -> Result<(), KeyError>,
) -> Result<[u8; 32], KeyError> {
    match read()? {
        Some(stored) => parse_key(&stored),
        None => {
            let key = mint()?;
            write(&to_hex(&key))?;
            Ok(key)
        }
    }
}

/// The OS keychain (`keyring` crate).
pub struct OsKeychain;

impl KeySource for OsKeychain {
    fn get_or_create(&self) -> Result<[u8; 32], KeyError> {
        let entry = keyring::Entry::new(SERVICE, USER).map_err(|_| KeyError::Unavailable)?;
        get_or_create_entry(
            || match entry.get_password() {
                Ok(s) => Ok(Some(s)),
                Err(keyring::Error::NoEntry) => Ok(None),
                Err(_) => Err(KeyError::Unavailable),
            },
            |s| entry.set_password(s).map_err(|_| KeyError::Unavailable),
        )
    }
}

#[cfg(test)]
pub struct MemoryKeychain {
    cell: std::sync::Mutex<Option<String>>,
    fail_next: std::sync::Mutex<bool>,
}

#[cfg(test)]
impl MemoryKeychain {
    pub fn new() -> Self {
        MemoryKeychain {
            cell: std::sync::Mutex::new(None),
            fail_next: std::sync::Mutex::new(false),
        }
    }

    /// Makes the next `get_or_create` report the keychain as gone.
    pub fn fail_next(&self) {
        *self.fail_next.lock().expect("fail_next lock") = true;
    }

    /// Preloads the entry (as if a previous launch wrote it).
    pub fn store(&self, value: &str) {
        *self.cell.lock().expect("cell lock") = Some(value.to_string());
    }
}

#[cfg(test)]
impl KeySource for MemoryKeychain {
    fn get_or_create(&self) -> Result<[u8; 32], KeyError> {
        get_or_create_entry(
            || {
                let mut fail = self.fail_next.lock().map_err(|_| KeyError::Unavailable)?;
                if *fail {
                    *fail = false;
                    return Err(KeyError::Unavailable);
                }
                Ok(self.cell.lock().map_err(|_| KeyError::Unavailable)?.clone())
            },
            |s| {
                *self.cell.lock().map_err(|_| KeyError::Unavailable)? = Some(s.to_string());
                Ok(())
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_once_then_returns_same_key() {
        let kc = MemoryKeychain::new();
        let a = kc.get_or_create().expect("first key");
        let b = kc.get_or_create().expect("second key");
        assert_eq!(a, b, "the stored key must be returned, not re-minted");
        assert!(!a.iter().all(|b| *b == 0), "a fresh key is not all zero");
    }

    #[test]
    fn unavailable_maps_to_keychain_unavailable() {
        let kc = MemoryKeychain::new();
        kc.fail_next();
        let err = kc.get_or_create().expect_err("keychain failure must error");
        assert_eq!(err.code(), "keychain_unavailable");
    }

    #[test]
    fn key_never_in_error_display() {
        // Whatever bytes the key holds, no error produced by this module
        // may contain them (spec §7.3: the key never appears in logs).
        let key = [0xabu8; 32];
        let hex = to_hex(&key);
        for err in [KeyError::Unavailable, KeyError::Malformed] {
            for rendered in [format!("{err}"), format!("{err:?}"), err.code().to_string()] {
                assert!(
                    !rendered.contains(&hex),
                    "error output contains key material: {rendered}"
                );
            }
        }
        // A malformed stored entry never echoes the stored bytes.
        let kc = MemoryKeychain::new();
        kc.store("not-hex");
        let err = kc.get_or_create().expect_err("malformed entry must error");
        assert_eq!(err.code(), "key_invalid");
        assert!(!format!("{err}").contains("not-hex"));
    }
}
