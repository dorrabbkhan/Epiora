//! Library surface of the desktop shell crate. The binary (`main.rs`)
//! keeps the Tauri setup; modules shared with integration tests live
//! here.

#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

pub mod legacy_sqlserver;
