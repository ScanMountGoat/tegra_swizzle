//! Documentation for the C API
use crate::BlockHeight;

/// Swizzles the bytes from `source` into `destination` using the block linear swizzling algorithm.
/// See the safe alternative [swizzle_block_linear](super::swizzle_block_linear).
/// # Safety
/// `source` and `source_len` should refer to an array with at least as many bytes as the result of [deswizzled_surface_size].
/// Similarly, `destination` and `destination_len` should refer to an array with at least as many bytes as as the result of [swizzled_surface_size].
///
/// `block_height` must be one of the supported values in [BlockHeight].
#[no_mangle]
pub unsafe extern "C" fn swizzle_block_linear(
    width: usize,
    height: usize,
    depth: usize,
    source: *const u8,
    source_len: usize,
    destination: *mut u8,
    destination_len: usize,
    block_height: usize,
    bytes_per_pixel: usize,
) {
    // TODO: Assert that the lengths are correct?
    let source = std::slice::from_raw_parts(source, source_len);
    let destination = std::slice::from_raw_parts_mut(destination, destination_len);

    crate::swizzle::swizzle_inner::<false>(
        width,
        height,
        depth,
        source,
        destination,
        BlockHeight::new(block_height).unwrap() as usize,
        depth,
        bytes_per_pixel,
    )
}

/// Deswizzles the bytes from `source` into `destination` using the block linear swizzling algorithm.
/// See the safe alternative [deswizzle_block_linear](super::deswizzle_block_linear).
/// # Safety
/// `source` and `source_len` should refer to an array with at least as many bytes as the result of [swizzled_surface_size].
/// Similarly, `destination` and `destination_len` should refer to an array with at least as many bytes as as the result of [deswizzled_surface_size].
///
/// `block_height` must be one of the supported values in [BlockHeight].
#[no_mangle]
pub unsafe extern "C" fn deswizzle_block_linear(
    width: usize,
    height: usize,
    depth: usize,
    source: *const u8,
    source_len: usize,
    destination: *mut u8,
    destination_len: usize,
    block_height: usize,
    bytes_per_pixel: usize,
) {
    let source = std::slice::from_raw_parts(source, source_len);
    let destination = std::slice::from_raw_parts_mut(destination, destination_len);

    crate::swizzle::swizzle_inner::<true>(
        width,
        height,
        depth,
        source,
        destination,
        BlockHeight::new(block_height).unwrap() as usize,
        depth,
        bytes_per_pixel,
    )
}

/// See [swizzled_surface_size](super::swizzled_surface_size).
/// `block_height` must be one of the supported values in [BlockHeight].
#[no_mangle]
pub extern "C" fn swizzled_surface_size(
    width: usize,
    height: usize,
    depth: usize,
    block_height: usize,
    bytes_per_pixel: usize,
) -> usize {
    super::swizzled_surface_size(
        width,
        height,
        depth,
        BlockHeight::new(block_height).unwrap(),
        bytes_per_pixel,
    )
}

/// See [deswizzled_surface_size](super::deswizzled_surface_size).
#[no_mangle]
pub extern "C" fn deswizzled_surface_size(
    width: usize,
    height: usize,
    depth: usize,
    bytes_per_pixel: usize,
) -> usize {
    super::deswizzled_surface_size(width, height, depth, bytes_per_pixel)
}

/// See [block_height_mip0](super::block_height_mip0).
#[no_mangle]
pub extern "C" fn block_height_mip0(height: usize) -> usize {
    super::block_height_mip0(height) as usize
}

/// See [mip_block_height](super::mip_block_height).
/// `block_height_mip0` must be one of the supported values in [BlockHeight].
#[no_mangle]
pub extern "C" fn mip_block_height(mip_height: usize, block_height_mip0: usize) -> usize {
    super::mip_block_height(mip_height, BlockHeight::new(block_height_mip0).unwrap()) as usize
}
