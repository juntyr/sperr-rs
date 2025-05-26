//! [![CI Status]][workflow] [![MSRV]][repo] [![Latest Version]][crates.io]
//! [![Rust Doc Crate]][docs.rs] [![Rust Doc Main]][docs]
//!
//! [CI Status]: https://img.shields.io/github/actions/workflow/status/juntyr/sperr-rs/ci.yml?branch=main
//! [workflow]: https://github.com/juntyr/sperr-rs/actions/workflows/ci.yml?query=branch%3Amain
//!
//! [MSRV]: https://img.shields.io/badge/MSRV-1.82.0-blue
//! [repo]: https://github.com/juntyr/sperr-rs
//!
//! [Latest Version]: https://img.shields.io/crates/v/sperr
//! [crates.io]: https://crates.io/crates/sperr
//!
//! [Rust Doc Crate]: https://img.shields.io/docsrs/sperr
//! [docs.rs]: https://docs.rs/sperr/
//!
//! [Rust Doc Main]: https://img.shields.io/badge/docs-main-blue
//! [docs]: https://juntyr.github.io/sperr-rs/sperr
//!
//! High-level bindigs to the [SPERR] compressor.
//!
//! [SPERR]: https://github.com/NCAR/SPERR

use std::ffi::c_int;

use ndarray::{ArrayView2, ArrayView3, ArrayViewMut2, ArrayViewMut3};

#[derive(Copy, Clone, PartialEq, Debug)]
/// SPERR compression mode / quality control
pub enum CompressionMode {
    /// Fixed bit-per-pixel rate
    BitsPerPixel {
        /// bits-per-pixel, must be in `0.0 <= bpp <= 64.0`
        bpp: f64,
    },
    /// Fixed peak signal-to-noise ratio
    PeakSignalToNoiseRatio {
        /// non-negative peak signal-to-noise ratio
        psnr: f64,
    },
    /// Fixed point-wise (absolute) error
    PointwiseError {
        /// non-negative point-wise (absolute) error
        pwe: f64,
    },
}

#[derive(Debug, thiserror::Error)]

/// Errors that can occur during compression and decompression with SPERR
pub enum Error {
    /// one or more parameters is invalid
    #[error("one or more parameters is invalid")]
    InvalidParameter,
    /// compressed data is missing the header
    #[error("compressed data is missing the header")]
    DecompressMissingHeader,
    /// cannot decompress to an array with a different shape
    #[error("cannot decompress to an array with a different shape")]
    DecompressShapeMismatch,
    /// other error
    #[error("other error")]
    Other,
}

impl CompressionMode {
    const fn as_mode(self) -> c_int {
        match self {
            Self::BitsPerPixel { .. } => 1,
            Self::PeakSignalToNoiseRatio { .. } => 2,
            Self::PointwiseError { .. } => 3,
        }
    }

    const fn as_quality(self) -> f64 {
        match self {
            Self::BitsPerPixel { bpp: quality }
            | Self::PeakSignalToNoiseRatio { psnr: quality }
            | Self::PointwiseError { pwe: quality } => quality,
        }
    }
}

/// Compress a 2d `src` slice of data with the compression `mode`
pub fn compress_2d<T: Element>(
    src: ArrayView2<T>,
    mode: CompressionMode,
) -> Result<Vec<u8>, Error> {
    let src = src.as_standard_layout();

    let mut dst = std::ptr::null_mut();
    let mut dst_len = 0;

    #[allow(unsafe_code)] // Safety: FFI
    let res = unsafe {
        sperr_sys::sperr_comp_2d(
            src.as_ptr().cast(),
            T::IS_FLOAT as _,
            src.dim().1,
            src.dim().0,
            mode.as_mode(),
            mode.as_quality(),
            true as _,
            std::ptr::addr_of_mut!(dst),
            std::ptr::addr_of_mut!(dst_len),
        )
    };

    match res {
        0 => (), // ok
        1 => unreachable!("sperr_comp_2d: dst is not pointing to a NULL pointer"),
        2 => return Err(Error::InvalidParameter),
        -1 => return Err(Error::Other),
        _ => unreachable!("sperr_comp_2d: unknown error kind {res}"),
    }

    #[allow(unsafe_code)] // Safety: dst is initialized by sperr_comp_2d
    let compressed =
        Vec::from(unsafe { std::slice::from_raw_parts(dst.cast_const().cast::<u8>(), dst_len) });

    #[allow(unsafe_code)] // Safety: FFI, dst is allocated by sperr_comp_2d
    unsafe {
        sperr_sys::free_dst(dst);
    }

    Ok(compressed)
}

/// Decompress a 2d SPERR-compressed `compressed` buffer into the `decompressed` array
pub fn decompress_into_2d<T: Element>(
    compressed: &[u8],
    mut decompressed: ArrayViewMut2<T>,
) -> Result<(), Error> {
    let Some((header, compressed)) = compressed.split_at_checked(10) else {
        return Err(Error::DecompressMissingHeader);
    };

    let mut dim_x = 0;
    let mut dim_y = 0;
    let mut dim_z = 0;
    let mut is_float = 0;

    #[allow(unsafe_code)] // Safety: FFI
    unsafe {
        sperr_sys::sperr_parse_header(
            header.as_ptr().cast(),
            std::ptr::addr_of_mut!(dim_x),
            std::ptr::addr_of_mut!(dim_y),
            std::ptr::addr_of_mut!(dim_z),
            std::ptr::addr_of_mut!(is_float),
        );
    }

    if (dim_z, dim_y, dim_x) != (1, decompressed.dim().0, decompressed.dim().1) {
        return Err(Error::DecompressShapeMismatch);
    }

    let mut dst = std::ptr::null_mut();

    #[allow(unsafe_code)] // Safety: FFI
    let res = unsafe {
        sperr_sys::sperr_decomp_2d(
            compressed.as_ptr().cast(),
            compressed.len(),
            T::IS_FLOAT as _,
            decompressed.dim().1,
            decompressed.dim().0,
            std::ptr::addr_of_mut!(dst),
        )
    };

    match res {
        0 => (), // ok
        1 => unreachable!("sperr_decomp_2d: dst is not pointing to a NULL pointer"),
        -1 => return Err(Error::Other),
        _ => unreachable!("sperr_decomp_2d: unknown error kind {res}"),
    }

    #[allow(unsafe_code)] // Safety: dst is initialized by sperr_decomp_2d
    let dec =
        unsafe { ArrayView2::from_shape_ptr(decompressed.dim(), dst.cast_const().cast::<T>()) };
    decompressed.assign(&dec);

    #[allow(unsafe_code)] // Safety: FFI, dst is allocated by sperr_decomp_2d
    unsafe {
        sperr_sys::free_dst(dst);
    }

    Ok(())
}

/// Compress a 3d `src` volume of data with the compression `mode` using the
/// preferred `chunks`
pub fn compress_3d<T: Element>(
    src: ArrayView3<T>,
    mode: CompressionMode,
    chunks: (usize, usize, usize),
) -> Result<Vec<u8>, Error> {
    let src = src.as_standard_layout();

    let mut dst = std::ptr::null_mut();
    let mut dst_len = 0;

    #[allow(unsafe_code)] // Safety: FFI
    let res = unsafe {
        sperr_sys::sperr_comp_3d(
            src.as_ptr().cast(),
            T::IS_FLOAT as _,
            src.dim().2,
            src.dim().1,
            src.dim().0,
            chunks.2,
            chunks.1,
            chunks.0,
            mode.as_mode(),
            mode.as_quality(),
            0,
            std::ptr::addr_of_mut!(dst),
            std::ptr::addr_of_mut!(dst_len),
        )
    };

    match res {
        0 => (), // ok
        1 => unreachable!("sperr_comp_3d: dst is not pointing to a NULL pointer"),
        2 => return Err(Error::InvalidParameter),
        -1 => return Err(Error::Other),
        _ => unreachable!("sperr_comp_3d: unknown error kind {res}"),
    }

    #[allow(unsafe_code)] // Safety: dst is initialized by sperr_comp_3d
    let compressed =
        Vec::from(unsafe { std::slice::from_raw_parts(dst.cast_const().cast::<u8>(), dst_len) });

    #[allow(unsafe_code)] // Safety: FFI, dst is allocated by sperr_comp_3d
    unsafe {
        sperr_sys::free_dst(dst);
    }

    Ok(compressed)
}

/// Decompress a 3d SPERR-compressed `compressed` buffer into the `decompressed` array
pub fn decompress_into_3d<T: Element>(
    compressed: &[u8],
    mut decompressed: ArrayViewMut3<T>,
) -> Result<(), Error> {
    let mut dim_x = 0;
    let mut dim_y = 0;
    let mut dim_z = 0;
    let mut is_float = 0;

    #[allow(unsafe_code)] // Safety: FFI
    unsafe {
        sperr_sys::sperr_parse_header(
            compressed.as_ptr().cast(),
            std::ptr::addr_of_mut!(dim_x),
            std::ptr::addr_of_mut!(dim_y),
            std::ptr::addr_of_mut!(dim_z),
            std::ptr::addr_of_mut!(is_float),
        );
    }

    if (dim_z, dim_y, dim_x)
        != (
            decompressed.dim().0,
            decompressed.dim().1,
            decompressed.dim().2,
        )
    {
        return Err(Error::DecompressShapeMismatch);
    }

    let mut dst = std::ptr::null_mut();

    #[allow(unsafe_code)] // Safety: FFI
    let res = unsafe {
        sperr_sys::sperr_decomp_3d(
            compressed.as_ptr().cast(),
            compressed.len(),
            T::IS_FLOAT as _,
            0,
            std::ptr::addr_of_mut!(dim_x),
            std::ptr::addr_of_mut!(dim_y),
            std::ptr::addr_of_mut!(dim_z),
            std::ptr::addr_of_mut!(dst),
        )
    };

    match res {
        0 => (), // ok
        1 => unreachable!("sperr_decomp_3d: dst is not pointing to a NULL pointer"),
        -1 => return Err(Error::Other),
        _ => unreachable!("sperr_decomp_3d: unknown error kind {res}"),
    }

    #[allow(unsafe_code)] // Safety: dst is initialized by sperr_decomp_3d
    let dec =
        unsafe { ArrayView3::from_shape_ptr(decompressed.dim(), dst.cast_const().cast::<T>()) };
    decompressed.assign(&dec);

    #[allow(unsafe_code)] // Safety: FFI, dst is allocated by sperr_decomp_3d
    unsafe {
        sperr_sys::free_dst(dst);
    }

    Ok(())
}

/// Marker trait for element types that can be compressed with SPERR
pub trait Element: sealed::Element {}

impl Element for f32 {}
impl sealed::Element for f32 {
    const IS_FLOAT: bool = true;
}

impl Element for f64 {}
impl sealed::Element for f64 {
    const IS_FLOAT: bool = false;
}

mod sealed {
    pub trait Element: Copy {
        const IS_FLOAT: bool;
    }
}
