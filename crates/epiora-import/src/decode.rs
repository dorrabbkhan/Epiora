//! Byte → text decoding (spec §6.1.2). UTF-8 is borrowed without a
//! copy when valid; other encodings are transcoded once. A single
//! leading U+FEFF is removed. `strict` decoding (an explicit
//! `settings.encoding`) turns decode loss into `encoding_invalid`;
//! auto-detected encodings decode leniently (detection already chose
//! them by BOM or UTF-8 validity).

use crate::error::ImportError;
use crate::settings::Encoding;
use std::borrow::Cow;

/// Decodes `bytes` under `encoding`.
pub(crate) fn decode(
    bytes: &[u8],
    encoding: Encoding,
    strict: bool,
) -> Result<Cow<'_, str>, ImportError> {
    let enc = match encoding {
        Encoding::Utf8 => encoding_rs::UTF_8,
        Encoding::Utf16le => encoding_rs::UTF_16LE,
        Encoding::Utf16be => encoding_rs::UTF_16BE,
        Encoding::Windows1252 => encoding_rs::WINDOWS_1252,
    };
    // `decode_without_bom_handling` keeps the BOM as U+FEFF, so BOM
    // semantics stay under our control: strip exactly one.
    let (cow, had_errors) = enc.decode_without_bom_handling(bytes);
    if strict && had_errors {
        return Err(ImportError::Encoding { encoding });
    }
    Ok(match cow {
        Cow::Borrowed(s) => Cow::Borrowed(s.strip_prefix('\u{FEFF}').unwrap_or(s)),
        Cow::Owned(s) => match s.strip_prefix('\u{FEFF}') {
            Some(t) => Cow::Owned(t.to_string()),
            None => Cow::Owned(s),
        },
    })
}
