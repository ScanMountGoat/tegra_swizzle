//! Documentation for the C API.
//!
//! For easier integration, none of the FFI methods allocate memory.
//! When tiling or untiling, make sure to allocate
//! the appropriate amount of memory for the destination array
//! by calling functions like [swizzled_surface_size] or [deswizzled_surface_size].
//!
//! For block height parameters, always use the result of [block_height_mip0]
//! or [mip_block_height] unless the format explicitly specifies a block height.
use crate::{surface::BlockDim, BlockHeight};

/// See [crate::surface::swizzle_surface].
///
/// # Safety
/// `source` and `source_len` should refer to an array with at least as many bytes as the result of [deswizzled_surface_size].
/// Similarly, `destination` and `destination_len` should refer to an array with at least as many bytes as as the result of [swizzled_surface_size].
///
/// All the fields of `block_dim` must be non zero.
///
/// `block_height` must be one of the supported values in [BlockHeight].
#[no_mangle]
pub unsafe extern "C" fn swizzle_surface(
    width: u32,
    height: u32,
    depth: u32,
    source: *const u8,
    source_len: usize,
    destination: *mut u8,
    destination_len: usize,
    block_dim: BlockDim,
    block_height_mip0: u32,
    bytes_per_pixel: u32,
    mipmap_count: u32,
    array_count: u32,
) {
    let source = std::slice::from_raw_parts(source, source_len);
    let mut destination = std::slice::from_raw_parts_mut(destination, destination_len);

    crate::surface::swizzle_surface_inner::<false>(
        width,
        height,
        depth,
        source,
        &mut destination,
        block_dim,
        Some(BlockHeight::new(block_height_mip0).unwrap()),
        bytes_per_pixel,
        mipmap_count,
        array_count,
    )
    .unwrap();
}

/// See [crate::surface::deswizzle_surface].
///
/// # Safety
/// `source` and `source_len` should refer to an array with at least as many bytes as the result of [swizzled_surface_size].
/// Similarly, `destination` and `destination_len` should refer to an array with at least as many bytes as as the result of [deswizzled_surface_size].
///
/// All the fields of `block_dim` must be non zero.
///
/// `block_height` must be one of the supported values in [BlockHeight].
#[no_mangle]
pub unsafe extern "C" fn deswizzle_surface(
    width: u32,
    height: u32,
    depth: u32,
    source: *const u8,
    source_len: usize,
    destination: *mut u8,
    destination_len: usize,
    block_dim: BlockDim,
    block_height_mip0: u32,
    bytes_per_pixel: u32,
    mipmap_count: u32,
    array_count: u32,
) {
    let source = std::slice::from_raw_parts(source, source_len);
    let mut destination = std::slice::from_raw_parts_mut(destination, destination_len);

    crate::surface::swizzle_surface_inner::<true>(
        width,
        height,
        depth,
        source,
        &mut destination,
        block_dim,
        Some(BlockHeight::new(block_height_mip0).unwrap()),
        bytes_per_pixel,
        mipmap_count,
        array_count,
    )
    .unwrap();
}

/// See [crate::surface::swizzle_surface].
///
/// # Safety
/// All the fields of `block_dim` must be non zero.
/// `block_height_mip0` must be one of the supported values in [BlockHeight].
#[no_mangle]
pub unsafe extern "C" fn swizzled_surface_size(
    width: u32,
    height: u32,
    depth: u32,
    block_dim: BlockDim,
    block_height_mip0: u32,
    bytes_per_pixel: u32,
    mipmap_count: u32,
    array_count: u32,
) -> usize {
    crate::surface::swizzled_surface_size(
        width,
        height,
        depth,
        block_dim,
        Some(BlockHeight::new(block_height_mip0).unwrap()),
        bytes_per_pixel,
        mipmap_count,
        array_count,
    )
}

/// See [crate::surface::swizzle_surface].
///
/// # Safety
/// All the fields of `block_dim` must be non zero.
#[no_mangle]
pub unsafe extern "C" fn deswizzled_surface_size(
    width: u32,
    height: u32,
    depth: u32,
    block_dim: BlockDim,
    bytes_per_pixel: u32,
    mipmap_count: u32,
    array_count: u32,
) -> usize {
    crate::surface::deswizzled_surface_size(
        width,
        height,
        depth,
        block_dim,
        bytes_per_pixel,
        mipmap_count,
        array_count,
    )
}

/// See [crate::swizzle::swizzle_block_linear].
///
/// # Safety
/// `source` and `source_len` should refer to an array with at least as many bytes as the result of [deswizzled_mip_size].
/// Similarly, `destination` and `destination_len` should refer to an array with at least as many bytes as as the result of [swizzled_mip_size].
///
/// `block_height` must be one of the supported values in [BlockHeight].
#[no_mangle]
pub unsafe extern "C" fn swizzle_block_linear(
    width: u32,
    height: u32,
    depth: u32,
    source: *const u8,
    source_len: usize,
    destination: *mut u8,
    destination_len: usize,
    block_height: u32,
    bytes_per_pixel: u32,
) {
    let source = std::slice::from_raw_parts(source, source_len);
    let destination = std::slice::from_raw_parts_mut(destination, destination_len);

    crate::swizzle::swizzle_inner::<false>(
        width,
        height,
        depth,
        source,
        destination,
        BlockHeight::new(block_height).unwrap(),
        depth,
        bytes_per_pixel,
    )
}

/// See [crate::swizzle::deswizzle_block_linear].
///
/// # Safety
/// `source` and `source_len` should refer to an array with at least as many bytes as the result of [swizzled_mip_size].
/// Similarly, `destination` and `destination_len` should refer to an array with at least as many bytes as as the result of [deswizzled_mip_size].
///
/// `block_height` must be one of the supported values in [BlockHeight].
#[no_mangle]
pub unsafe extern "C" fn deswizzle_block_linear(
    width: u32,
    height: u32,
    depth: u32,
    source: *const u8,
    source_len: usize,
    destination: *mut u8,
    destination_len: usize,
    block_height: u32,
    bytes_per_pixel: u32,
) {
    let source = std::slice::from_raw_parts(source, source_len);
    let destination = std::slice::from_raw_parts_mut(destination, destination_len);

    crate::swizzle::swizzle_inner::<true>(
        width,
        height,
        depth,
        source,
        destination,
        BlockHeight::new(block_height).unwrap(),
        depth,
        bytes_per_pixel,
    )
}

/// See [crate::swizzle::swizzled_mip_size].
///
/// # Safety
/// `block_height` must be one of the supported values in [BlockHeight].
#[no_mangle]
pub unsafe extern "C" fn swizzled_mip_size(
    width: u32,
    height: u32,
    depth: u32,
    block_height: u32,
    bytes_per_pixel: u32,
) -> usize {
    crate::swizzle::swizzled_mip_size(
        width,
        height,
        depth,
        BlockHeight::new(block_height).unwrap(),
        bytes_per_pixel,
    )
}

/// See [crate::swizzle::deswizzled_mip_size].
#[no_mangle]
pub extern "C" fn deswizzled_mip_size(
    width: u32,
    height: u32,
    depth: u32,
    bytes_per_pixel: u32,
) -> usize {
    crate::swizzle::deswizzled_mip_size(width, height, depth, bytes_per_pixel)
}

/// See [crate::block_height_mip0].
#[no_mangle]
pub extern "C" fn block_height_mip0(height: u32) -> u32 {
    super::block_height_mip0(height) as u32
}

/// See [crate::mip_block_height].
///
/// # Safety
/// `block_height_mip0` must be one of the supported values in [BlockHeight].
#[no_mangle]
pub unsafe extern "C" fn mip_block_height(mip_height: u32, block_height_mip0: u32) -> u32 {
    super::mip_block_height(mip_height, BlockHeight::new(block_height_mip0).unwrap()) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    use alloc::vec;

    #[test]
    fn swizzle_surface_rgba_16_16_16() {
        let input = include_bytes!("../block_linear/16_16_16_rgba.bin");
        let expected = include_bytes!("../block_linear/16_16_16_rgba_tiled.bin");

        let block_height = block_height_mip0(16);
        let size =
            unsafe { deswizzled_surface_size(16, 16, 16, BlockDim::uncompressed(), 4, 1, 1) };
        let mut actual = vec![0u8; size];
        unsafe {
            swizzle_surface(
                16,
                16,
                16,
                input.as_ptr(),
                input.len(),
                actual.as_mut_ptr(),
                actual.len(),
                BlockDim::uncompressed(),
                block_height,
                4,
                1,
                1,
            );
        }
        assert_eq!(expected, &actual[..]);
    }

    #[test]
    fn deswizzle_surface_rgba_16_16_16() {
        let input = include_bytes!("../block_linear/16_16_16_rgba_tiled.bin");
        let expected = include_bytes!("../block_linear/16_16_16_rgba.bin");

        let block_height = block_height_mip0(16);
        let size = unsafe {
            swizzled_surface_size(16, 16, 16, BlockDim::uncompressed(), block_height, 4, 1, 1)
        };
        let mut actual = vec![0u8; size];
        unsafe {
            deswizzle_surface(
                16,
                16,
                16,
                input.as_ptr(),
                input.len(),
                actual.as_mut_ptr(),
                actual.len(),
                BlockDim::uncompressed(),
                block_height,
                4,
                1,
                1,
            );
        }
        assert_eq!(expected, &actual[..]);
    }

    #[test]
    fn swizzle_rgba_16_16_16() {
        let input = include_bytes!("../block_linear/16_16_16_rgba.bin");
        let expected = include_bytes!("../block_linear/16_16_16_rgba_tiled.bin");

        let size = unsafe { swizzled_mip_size(16, 16, 16, 1, 4) };
        let mut actual = vec![0u8; size];
        unsafe {
            swizzle_block_linear(
                16,
                16,
                16,
                input.as_ptr(),
                input.len(),
                actual.as_mut_ptr(),
                actual.len(),
                1,
                4,
            );
        }

        assert_eq!(expected, &actual[..]);
    }

    #[test]
    fn deswizzle_rgba_16_16_16() {
        let input = include_bytes!("../block_linear/16_16_16_rgba_tiled.bin");
        let expected = include_bytes!("../block_linear/16_16_16_rgba.bin");

        let size = deswizzled_mip_size(16, 16, 16, 4);
        let mut actual = vec![0u8; size];
        unsafe {
            deswizzle_block_linear(
                16,
                16,
                16,
                input.as_ptr(),
                input.len(),
                actual.as_mut_ptr(),
                actual.len(),
                1,
                4,
            );
        }

        assert_eq!(expected, &actual[..]);
    }

    #[test]
    fn mip_block_height_bcn() {
        assert_eq!(4, unsafe {
            mip_block_height(128 / 4, block_height_mip0(128 / 4))
        });
    }
}
