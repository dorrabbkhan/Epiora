//! `special` — the mathematical functions of spec §3.4, all through
//! `libm` so native and wasm builds are bit-identical. `normal` and
//! `hypergeom` are ports of R's own nmath routines; `gamma`/`beta` use
//! the series / continued-fraction forms the spec names; `root` is
//! Brent's method.

mod beta;
mod chisq;
mod gamma;
mod hypergeom;
mod normal;
mod root;

pub use beta::{beta_quantile, reg_inc_beta};
pub use chisq::chisq_sf;
pub use gamma::ln_gamma;
pub use hypergeom::{binomial_tail, hypergeom_pmf, NcHypergeom};
pub use normal::{normal_cdf, normal_quantile, normal_sf};
pub use root::{brent_root, RootError};
