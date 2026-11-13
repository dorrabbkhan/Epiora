//! `special` — the mathematical functions of spec §3.4, all through
//! `libm` so native and wasm builds are bit-identical. `normal` and
//! `hypergeom` are ports of R's own nmath routines; `gamma`/`beta` use
//! the series / continued-fraction forms the spec names; `root` is
//! Brent's method. `t_dist`, `f_dist`, `gamma_quantile` and `discrete`
//! are the v1.0 spec §4.1 additions on those building blocks.

mod beta;
mod chisq;
mod discrete;
mod f_dist;
mod gamma;
mod gamma_quantile;
mod hypergeom;
mod normal;
mod root;
mod t_dist;

pub use beta::{beta_quantile, reg_inc_beta};
pub use chisq::{chisq_cdf, chisq_sf};
pub use discrete::{binomial_quantile, poisson_tail};
pub use f_dist::f_sf;
pub use gamma::ln_gamma;
pub use gamma_quantile::{chisq_quantile, gamma_quantile};
pub use hypergeom::{binomial_tail, hypergeom_pmf, NcHypergeom};
pub use normal::{normal_cdf, normal_quantile, normal_sf};
pub use root::{brent_root, RootError};
pub use t_dist::{t_cdf, t_quantile, t_sf};
