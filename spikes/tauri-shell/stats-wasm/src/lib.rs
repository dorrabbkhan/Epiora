//! Rust statistics function compiled to wasm for the Tauri shell spike.
//!
//! `risk_ratio(a, b, c, d)` treats the four cells of a 2x2 table as
//! `a`/`b` = ill/well among exposed and `c`/`d` = ill/well among
//! unexposed, and returns the risk ratio
//! `(a / (a + b)) / (c / (c + d))`.
//!
//! The signature is the one the plan fixes:
//! `risk_ratio(a: f64, b: f64, c: f64, d: f64) -> f64`.
//! Exported without wasm-bindgen so the page can call it straight off
//! `WebAssembly.instantiate` with plain f64 arguments.

#[no_mangle]
pub extern "C" fn risk_ratio(a: f64, b: f64, c: f64, d: f64) -> f64 {
    (a / (a + b)) / (c / (c + d))
}

#[cfg(test)]
mod tests {
    use super::risk_ratio;

    #[test]
    fn oswego_2x2_matches_fixture() {
        // Oswego: ate vanilla ice cream x ill — 43/11 vs 3/18.
        // R fixture (epitools::riskratio, fixtures/2x2/oswego_vanilla.json):
        // attack rates 79.6% vs 14.3%, RR rounded to 2 decimals = 5.57.
        let rr = risk_ratio(43.0, 11.0, 3.0, 18.0);
        assert!((rr - 5.574_074_074_074_074).abs() < 1e-9);
        assert_eq!(format!("{rr:.2}"), "5.57");
    }
}
