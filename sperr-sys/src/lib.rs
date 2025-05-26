//! [![CI Status]][workflow] [![MSRV]][repo] [![Latest Version]][crates.io]
//! [![Rust Doc Crate]][docs.rs] [![Rust Doc Main]][docs]
//!
//! [CI Status]: https://img.shields.io/github/actions/workflow/status/juntyr/tthresh-rs/ci.yml?branch=main
//! [workflow]: https://github.com/juntyr/tthresh-rs/actions/workflows/ci.yml?query=branch%3Amain
//!
//! [MSRV]: https://img.shields.io/badge/MSRV-1.82.0-blue
//! [repo]: https://github.com/juntyr/tthresh-rs
//!
//! [Latest Version]: https://img.shields.io/crates/v/tthresh-sys
//! [crates.io]: https://crates.io/crates/tthresh-sys
//!
//! [Rust Doc Crate]: https://img.shields.io/docsrs/tthresh-sys
//! [docs.rs]: https://docs.rs/tthresh-sys/
//!
//! [Rust Doc Main]: https://img.shields.io/badge/docs-main-blue
//! [docs]: https://juntyr.github.io/tthresh-rs/tthresh_sys
//!
//! Low-level bindigs to the [tthresh] compressor.
//!
//! [tthresh]: https://github.com/rballester/tthresh

#![allow(missing_docs)] // bindgen
#![allow(unsafe_code)] // sys-crate
#![allow(non_upper_case_globals, non_camel_case_types)] // bindgen

#[cfg(feature = "openmp")]
use ::openmp_sys as _;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
