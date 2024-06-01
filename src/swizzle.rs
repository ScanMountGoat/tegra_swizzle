//! Functions for tiling and untiling a single mipmap of a surface.
//!
//! These functions are for advanced usages of tiling and untiling.
//! Most texture formats should use the surface functions
//! to handle mipmap and array layer alignment.
use crate::{
    blockdepth::block_depth, div_round_up, height_in_blocks, round_up, width_in_gobs, BlockHeight,
    SwizzleError, GOB_HEIGHT_IN_BYTES, GOB_SIZE_IN_BYTES, GOB_WIDTH_IN_BYTES,
};

/// Tiles the bytes from `source` using the block linear algorithm.
///
/// Returns [SwizzleError::NotEnoughData] if `source` does not have
/// at least as many bytes as the result of [deswizzled_mip_size].
///
/// # Examples
/// Uncompressed formats like R8G8B8A8 can use the width and height in pixels.
/**
```rust
use tegra_swizzle::{block_height_mip0, swizzle::deswizzled_mip_size, swizzle::swizzle_block_linear};

let width = 512;
let height = 512;
let block_height = block_height_mip0(height);
# let size = deswizzled_mip_size(width, height, 1, 4);
# let input = vec![0u8; size];
let output = swizzle_block_linear(width, height, 1, &input, block_height, 4);
```
 */
/// For compressed formats with multiple pixels in a block, divide the width and height by the block dimensions.
/**
```rust
# use tegra_swizzle::{swizzle::deswizzled_mip_size, swizzle::swizzle_block_linear};
// BC7 has 4x4 pixel blocks that each take up 16 bytes.
use tegra_swizzle::{block_height_mip0, div_round_up};

let width = 512;
let height = 512;
let block_height = block_height_mip0(div_round_up(height, 4));
# let size = deswizzled_mip_size(div_round_up(width, 4), div_round_up(height, 4), 1, 16);
# let input = vec![0u8; size];
let output = swizzle_block_linear(
    div_round_up(width, 4),
    div_round_up(height, 4),
    1,
    &input,
    block_height,
    16,
);
```
 */
pub fn swizzle_block_linear(
    width: usize,
    height: usize,
    depth: usize,
    source: &[u8],
    block_height: BlockHeight,
    bytes_per_pixel: usize,
) -> Result<Vec<u8>, SwizzleError> {
    let mut destination =
        vec![0u8; swizzled_mip_size(width, height, depth, block_height, bytes_per_pixel)];

    let expected_size = deswizzled_mip_size(width, height, depth, bytes_per_pixel);
    if source.len() < expected_size {
        return Err(SwizzleError::NotEnoughData {
            actual_size: source.len(),
            expected_size,
        });
    }

    // TODO: This should be a parameter since it varies by mipmap?
    let block_depth = block_depth(depth);

    swizzle_inner::<false>(
        width,
        height,
        depth,
        source,
        &mut destination,
        block_height,
        block_depth,
        bytes_per_pixel,
    );
    Ok(destination)
}

/// Untiles the bytes from `source` using the block linear algorithm.
///
/// Returns [SwizzleError::NotEnoughData] if `source` does not have
/// at least as many bytes as the result of [swizzled_mip_size].
///
/// # Examples
/// Uncompressed formats like R8G8B8A8 can use the width and height in pixels.
/**
```rust
use tegra_swizzle::{block_height_mip0, swizzle::swizzled_mip_size, swizzle::deswizzle_block_linear};

let width = 512;
let height = 512;
let block_height = block_height_mip0(height);
# let size = swizzled_mip_size(width, height, 1, block_height, 4);
# let input = vec![0u8; size];
let output = deswizzle_block_linear(width, height, 1, &input, block_height, 4);
```
 */
/// For compressed formats with multiple pixels in a block, divide the width and height by the block dimensions.
/**
```rust
# use tegra_swizzle::{BlockHeight, swizzle::swizzled_mip_size, swizzle::deswizzle_block_linear};
// BC7 has 4x4 pixel blocks that each take up 16 bytes.
use tegra_swizzle::{block_height_mip0, div_round_up};

let width = 512;
let height = 512;
let block_height = block_height_mip0(div_round_up(height, 4));
# let size = swizzled_mip_size(div_round_up(width, 4), div_round_up(height, 4), 1, BlockHeight::Sixteen, 16);
# let input = vec![0u8; size];
let output = deswizzle_block_linear(
    div_round_up(width, 4),
    div_round_up(height, 4),
    1,
    &input,
    block_height,
    16,
);
```
 */
pub fn deswizzle_block_linear(
    width: usize,
    height: usize,
    depth: usize,
    source: &[u8],
    block_height: BlockHeight,
    bytes_per_pixel: usize,
) -> Result<Vec<u8>, SwizzleError> {
    let mut destination = vec![0u8; deswizzled_mip_size(width, height, depth, bytes_per_pixel)];

    let expected_size = swizzled_mip_size(width, height, depth, block_height, bytes_per_pixel);
    if source.len() < expected_size {
        return Err(SwizzleError::NotEnoughData {
            actual_size: source.len(),
            expected_size,
        });
    }

    // TODO: This should be a parameter since it varies by mipmap?
    let block_depth = block_depth(depth);

    swizzle_inner::<true>(
        width,
        height,
        depth,
        source,
        &mut destination,
        block_height,
        block_depth,
        bytes_per_pixel,
    );
    Ok(destination)
}

pub(crate) fn swizzle_inner<const DESWIZZLE: bool>(
    width: usize,
    height: usize,
    depth: usize,
    source: &[u8],
    destination: &mut [u8],
    block_height: BlockHeight,
    block_depth: usize,
    bytes_per_pixel: usize,
) {
    let block_height = block_height as usize;
    let width_in_gobs = width_in_gobs(width, bytes_per_pixel);

    let slice_size = slice_size(block_height, block_depth, width_in_gobs, height);

    // Blocks are always one GOB wide.
    // TODO: Citation?
    let block_width = 1;
    let block_size_in_bytes = GOB_SIZE_IN_BYTES * block_width * block_height * block_depth;
    let block_height_in_bytes = GOB_HEIGHT_IN_BYTES * block_height;

    // Tiling is defined as a mapping from byte coordinates x,y,z -> x',y',z'.
    // We step a GOB of bytes at a time to optimize the inner loop with SIMD loads/stores.
    // GOBs always use the same tiling patterns, so we can optimize tiling complete 64x8 GOBs.
    // The partially filled GOBs along the right and bottom edge use a slower per byte implementation.
    for z0 in 0..depth {
        let offset_z = gob_address_z(z0, block_height, block_depth, slice_size);

        // Step by a GOB of bytes in y.
        for y0 in (0..height).step_by(GOB_HEIGHT_IN_BYTES) {
            let offset_y = gob_address_y(
                y0,
                block_height_in_bytes,
                block_size_in_bytes,
                width_in_gobs,
            );

            // Step by a GOB of bytes in x.
            // The bytes per pixel converts pixel coordinates to byte coordinates.
            // This assumes BCN formats pass in their width and height in number of blocks rather than pixels.
            for x0 in (0..(width * bytes_per_pixel)).step_by(GOB_WIDTH_IN_BYTES) {
                let offset_x = gob_address_x(x0, block_size_in_bytes);

                let gob_address = offset_z + offset_y + offset_x;

                // Check if we can use the fast path.
                if x0 + GOB_WIDTH_IN_BYTES < width * bytes_per_pixel
                    && y0 + GOB_HEIGHT_IN_BYTES < height
                {
                    let linear_offset = (z0 * width * height * bytes_per_pixel)
                        + (y0 * width * bytes_per_pixel)
                        + x0;

                    // Use optimized code to reassign bytes.
                    if DESWIZZLE {
                        deswizzle_complete_gob(
                            &mut destination[linear_offset..],
                            &source[gob_address..],
                            width * bytes_per_pixel,
                        );
                    } else {
                        swizzle_complete_gob(
                            &mut destination[gob_address..],
                            &source[linear_offset..],
                            width * bytes_per_pixel,
                        );
                    }
                } else {
                    // There may be a row and column with partially filled GOBs.
                    // Fall back to a slow implementation that iterates over each byte.
                    swizzle_deswizzle_gob::<DESWIZZLE>(
                        destination,
                        source,
                        x0,
                        y0,
                        z0,
                        width,
                        height,
                        bytes_per_pixel,
                        gob_address,
                    );
                }
            }
        }
    }
}

fn swizzle_deswizzle_gob<const DESWIZZLE: bool>(
    destination: &mut [u8],
    source: &[u8],
    x0: usize,
    y0: usize,
    z0: usize,
    width: usize,
    height: usize,
    bytes_per_pixel: usize,
    gob_address: usize,
) {
    for y in 0..GOB_HEIGHT_IN_BYTES {
        for x in 0..GOB_WIDTH_IN_BYTES {
            if y0 + y < height && x0 + x < width * bytes_per_pixel {
                let swizzled_offset = gob_address + gob_offset(x, y);
                let linear_offset = (z0 * width * height * bytes_per_pixel)
                    + ((y0 + y) * width * bytes_per_pixel)
                    + x0
                    + x;

                // Swap the addresses for tiling vs untiling.
                if DESWIZZLE {
                    destination[linear_offset] = source[swizzled_offset];
                } else {
                    destination[swizzled_offset] = source[linear_offset];
                }
            }
        }
    }
}

// The gob address and slice size functions are ported from Ryujinx Emulator.
// https://github.com/Ryujinx/Ryujinx/blob/master/Ryujinx.Graphics.Texture/BlockLinearLayout.cs
// License MIT: https://github.com/Ryujinx/Ryujinx/blob/master/LICENSE.txt.
fn slice_size(
    block_height: usize,
    block_depth: usize,
    width_in_gobs: usize,
    height: usize,
) -> usize {
    let rob_size = GOB_SIZE_IN_BYTES * block_height * block_depth * width_in_gobs;
    div_round_up(height, block_height * GOB_HEIGHT_IN_BYTES) * rob_size
}

fn gob_address_z(z: usize, block_height: usize, block_depth: usize, slice_size: usize) -> usize {
    // Each "column" of blocks has block_depth many blocks.
    // A 16x16x16 RGBA8 3d texture has the following untiled GOB indices.
    //  0, 16,
    //  1, 17,
    // ...
    // 14, 30
    // 15, 31
    (z / block_depth * slice_size) + ((z & (block_depth - 1)) * GOB_SIZE_IN_BYTES * block_height)
}

fn gob_address_y(
    y: usize,
    block_height_in_bytes: usize,
    block_size_in_bytes: usize,
    image_width_in_gobs: usize,
) -> usize {
    let block_y = y / block_height_in_bytes;
    let block_inner_row = y % block_height_in_bytes / GOB_HEIGHT_IN_BYTES;
    block_y * block_size_in_bytes * image_width_in_gobs + block_inner_row * GOB_SIZE_IN_BYTES
}

// Code for offset_x and offset_y adapted from examples in the Tegra TRM v1.3 page 1217.
fn gob_address_x(x: usize, block_size_in_bytes: usize) -> usize {
    let block_x = x / GOB_WIDTH_IN_BYTES;
    block_x * block_size_in_bytes
}

// Code taken from examples in Tegra TRM v1.3 page 1218.
// Return the offset within the GOB for the byte at location (x, y).
fn gob_offset(x: usize, y: usize) -> usize {
    // TODO: Optimize this?
    // TODO: Describe the pattern here?
    ((x % 64) / 32) * 256 + ((y % 8) / 2) * 64 + ((x % 32) / 16) * 32 + (y % 2) * 16 + (x % 16)
}

// TODO: Investigate using macros to generate this code.
// TODO: Is it faster to use 16 byte loads for each row on incomplete GOBs?
// This may lead to better performance if the GOB is almost complete.

const GOB_ROW_OFFSETS: [usize; GOB_HEIGHT_IN_BYTES] = [0, 16, 64, 80, 128, 144, 192, 208];

// An optimized version of the gob_offset for an entire GOB worth of bytes.
// The tiled GOB is a contiguous region of 512 bytes.
// The untiled GOB is a 64x8 2D region of memory, so we need to account for the pitch.
fn deswizzle_complete_gob(dst: &mut [u8], src: &[u8], row_size_in_bytes: usize) {
    // Hard code each of the GOB_HEIGHT many rows.
    // This allows the compiler to optimize the copies with SIMD instructions.
    for (i, offset) in GOB_ROW_OFFSETS.iter().enumerate() {
        deswizzle_gob_row(dst, row_size_in_bytes * i, src, *offset);
    }
}

fn deswizzle_gob_row(dst: &mut [u8], dst_offset: usize, src: &[u8], src_offset: usize) {
    let dst = &mut dst[dst_offset..];
    let src = &src[src_offset..];
    // Start with the largest offset first to reduce bounds checks.
    dst[48..64].copy_from_slice(&src[288..304]);
    dst[32..48].copy_from_slice(&src[256..272]);
    dst[16..32].copy_from_slice(&src[32..48]);
    dst[0..16].copy_from_slice(&src[0..16]);
}

// The swizzle functions are identical but with the addresses swapped.
fn swizzle_complete_gob(dst: &mut [u8], src: &[u8], row_size_in_bytes: usize) {
    for (i, offset) in GOB_ROW_OFFSETS.iter().enumerate() {
        swizzle_gob_row(dst, *offset, src, row_size_in_bytes * i);
    }
}

fn swizzle_gob_row(dst: &mut [u8], dst_offset: usize, src: &[u8], src_offset: usize) {
    let dst = &mut dst[dst_offset..];
    let src = &src[src_offset..];
    dst[288..304].copy_from_slice(&src[48..64]);
    dst[256..272].copy_from_slice(&src[32..48]);
    dst[32..48].copy_from_slice(&src[16..32]);
    dst[0..16].copy_from_slice(&src[0..16]);
}

/// Calculates the size in bytes for the tiled data for the given dimensions for the block linear format.
///
/// The result of [swizzled_mip_size] will always be aligned to the GOB size of 512 bytes.
/// The result will be at least as large as [deswizzled_mip_size]
/// for the same surface parameters.
///
/// # Examples
/// Uncompressed formats like R8G8B8A8 can use the width and height in pixels.
/**
```rust
use tegra_swizzle::{block_height_mip0, swizzle::swizzled_mip_size};

let width = 256;
let height = 256;
let block_height = block_height_mip0(height);
assert_eq!(262144, swizzled_mip_size(width, height, 1, block_height, 4));
```
 */
/// For compressed formats with multiple pixels in a block, divide the width and height by the block dimensions.
/**
```rust
# use tegra_swizzle::{swizzle::swizzled_mip_size};
// BC7 has 4x4 pixel blocks that each take up 16 bytes.
use tegra_swizzle::{block_height_mip0, div_round_up};

let width = 256;
let height = 256;
let block_height = block_height_mip0(div_round_up(height, 4));
assert_eq!(
    65536,
    swizzled_mip_size(
        div_round_up(width, 4),
        div_round_up(height, 4),
        1,
        block_height,
        16
    )
);
```
 */
pub const fn swizzled_mip_size(
    width: usize,
    height: usize,
    depth: usize,
    block_height: BlockHeight,
    bytes_per_pixel: usize,
) -> usize {
    // Assume each block is 1 GOB wide.
    let width_in_gobs = width_in_gobs(width, bytes_per_pixel);

    let height_in_blocks = height_in_blocks(height, block_height as usize);
    let height_in_gobs = height_in_blocks * block_height as usize;

    let depth_in_gobs = round_up(depth, block_depth(depth));

    let num_gobs = width_in_gobs * height_in_gobs * depth_in_gobs;
    num_gobs * GOB_SIZE_IN_BYTES
}

/// Calculates the size in bytes for the untiled or linear data for the given dimensions.
///
/// # Examples
/// Uncompressed formats like R8G8B8A8 can use the width and height in pixels.
/**
```rust
use tegra_swizzle::{BlockHeight, swizzle::deswizzled_mip_size};

let width = 256;
let height = 256;
assert_eq!(262144, deswizzled_mip_size(width, height, 1, 4));
```
 */
/// For compressed formats with multiple pixels in a block, divide the width and height by the block dimensions.
/**
```rust
# use tegra_swizzle::{BlockHeight, swizzle::deswizzled_mip_size};
// BC7 has 4x4 pixel blocks that each take up 16 bytes.
use tegra_swizzle::div_round_up;

let width = 256;
let height = 256;
assert_eq!(
    65536,
    deswizzled_mip_size(div_round_up(width, 4), div_round_up(height, 4), 1, 16)
);
```
 */
pub const fn deswizzled_mip_size(
    width: usize,
    height: usize,
    depth: usize,
    bytes_per_pixel: usize,
) -> usize {
    width * height * depth * bytes_per_pixel
}

#[cfg(test)]
mod tests {
    use super::*;

    use rand::{rngs::StdRng, Rng, SeedableRng};

    #[test]
    fn swizzle_deswizzle_bytes_per_pixel() {
        let width = 312;
        let height = 575;
        let block_height = BlockHeight::Eight;

        // Test a value that isn't 4, 8, or 16.
        // Non standard values won't show up in practice.
        // The tiling algorithm should still handle these cases.
        let bytes_per_pixel = 12;

        let deswizzled_size = deswizzled_mip_size(width, height, 1, bytes_per_pixel);

        // Generate mostly unique input data.
        let seed = [13u8; 32];
        let mut rng: StdRng = SeedableRng::from_seed(seed);
        let input: Vec<_> = (0..deswizzled_size)
            .map(|_| rng.gen_range::<u8, _>(0..=255))
            .collect();

        let swizzled =
            swizzle_block_linear(width, height, 1, &input, block_height, bytes_per_pixel).unwrap();

        let deswizzled =
            deswizzle_block_linear(width, height, 1, &swizzled, block_height, bytes_per_pixel)
                .unwrap();

        assert_eq!(input, deswizzled);
    }

    #[test]
    fn swizzle_empty() {
        let result = swizzle_block_linear(32, 32, 1, &[], BlockHeight::Sixteen, 4);
        assert!(matches!(
            result,
            Err(SwizzleError::NotEnoughData {
                actual_size: 0,
                expected_size: 4096
            })
        ));
    }

    #[test]
    fn deswizzle_empty() {
        let result = deswizzle_block_linear(32, 32, 1, &[], BlockHeight::Sixteen, 4);
        assert!(matches!(
            result,
            Err(SwizzleError::NotEnoughData {
                actual_size: 0,
                expected_size: 16384
            })
        ));
    }

    #[test]
    fn swizzle_bc7_64_64_not_enough_data() {
        let result = swizzle_block_linear(
            64 / 4,
            64 / 4,
            1,
            &vec![0u8; 64 * 64 - 1],
            BlockHeight::Sixteen,
            16,
        );
        assert!(matches!(
            result,
            Err(SwizzleError::NotEnoughData {
                actual_size: 4095,
                expected_size: 4096
            })
        ));
    }

    #[test]
    fn deswizzle_bc7_64_64_not_enough_data() {
        let result =
            deswizzle_block_linear(64 / 4, 64 / 4, 1, &[0u8; 64 * 64], BlockHeight::Sixteen, 16);
        assert!(matches!(
            result,
            Err(SwizzleError::NotEnoughData {
                actual_size: 4096,
                expected_size: 32768
            })
        ));
    }

    #[test]
    fn swizzle_deswizzle_bc7_64_64() {
        // Test an even size.
        let swizzled = include_bytes!("../block_linear/64_bc7_tiled.bin");
        let deswizzled =
            deswizzle_block_linear(64 / 4, 64 / 4, 1, swizzled, BlockHeight::Two, 16).unwrap();

        let new_swizzled =
            swizzle_block_linear(64 / 4, 64 / 4, 1, &deswizzled, BlockHeight::Two, 16).unwrap();
        assert_eq!(swizzled, &new_swizzled[..]);
    }

    #[test]
    fn deswizzle_bc7_64_64() {
        let input = include_bytes!("../block_linear/64_bc7_tiled.bin");
        let expected = include_bytes!("../block_linear/64_bc7.bin");
        let actual =
            deswizzle_block_linear(64 / 4, 64 / 4, 1, input, BlockHeight::Two, 16).unwrap();

        assert_eq!(expected, &actual[..]);
    }

    #[test]
    fn deswizzle_bc1_128_128() {
        let input = include_bytes!("../block_linear/128_bc1_tiled.bin");
        let expected = include_bytes!("../block_linear/128_bc1.bin");
        let actual =
            deswizzle_block_linear(128 / 4, 128 / 4, 1, input, BlockHeight::Four, 8).unwrap();

        assert_eq!(expected, &actual[..]);
    }

    #[test]
    fn deswizzle_bc3_128_128() {
        let input = include_bytes!("../block_linear/128_bc3_tiled.bin");
        let expected = include_bytes!("../block_linear/128_bc3.bin");
        let actual =
            deswizzle_block_linear(128 / 4, 128 / 4, 1, input, BlockHeight::Four, 16).unwrap();

        assert_eq!(expected, &actual[..]);
    }

    #[test]
    fn deswizzle_rgba_f32_128_128() {
        let input = include_bytes!("../block_linear/128_rgbaf32_tiled.bin");
        let expected = include_bytes!("../block_linear/128_rgbaf32.bin");
        let actual = deswizzle_block_linear(128, 128, 1, input, BlockHeight::Sixteen, 16).unwrap();

        assert_eq!(expected, &actual[..]);
    }

    #[test]
    fn deswizzle_bc7_128_128() {
        let input = include_bytes!("../block_linear/128_bc7_tiled.bin");
        let expected = include_bytes!("../block_linear/128_bc7.bin");
        let actual =
            deswizzle_block_linear(128 / 4, 128 / 4, 1, input, BlockHeight::Four, 16).unwrap();

        assert_eq!(expected, &actual[..]);
    }

    #[test]
    fn deswizzle_bc7_256_256() {
        let input = include_bytes!("../block_linear/256_bc7_tiled.bin");
        let expected = include_bytes!("../block_linear/256_bc7.bin");
        let actual =
            deswizzle_block_linear(256 / 4, 256 / 4, 1, input, BlockHeight::Eight, 16).unwrap();

        assert_eq!(expected, &actual[..]);
    }

    #[test]
    fn deswizzle_bc7_320_320() {
        let input = include_bytes!("../block_linear/320_bc7_tiled.bin");
        let expected = include_bytes!("../block_linear/320_bc7.bin");
        let actual =
            deswizzle_block_linear(320 / 4, 320 / 4, 1, input, BlockHeight::Eight, 16).unwrap();

        assert_eq!(expected, &actual[..]);
    }

    #[test]
    fn deswizzle_bc7_512_512() {
        let input = include_bytes!("../block_linear/512_bc7_tiled.bin");
        let expected = include_bytes!("../block_linear/512_bc7.bin");
        let actual =
            deswizzle_block_linear(512 / 4, 512 / 4, 1, input, BlockHeight::Sixteen, 16).unwrap();

        assert_eq!(expected, &actual[..]);
    }

    #[test]
    fn deswizzle_bc7_1024_1024() {
        let input = include_bytes!("../block_linear/1024_bc7_tiled.bin");
        let expected = include_bytes!("../block_linear/1024_bc7.bin");
        let actual =
            deswizzle_block_linear(1024 / 4, 1024 / 4, 1, input, BlockHeight::Sixteen, 16).unwrap();

        assert_eq!(expected, &actual[..]);
    }

    #[test]
    fn deswizzle_rgba_16_16_16() {
        let input = include_bytes!("../block_linear/16_16_16_rgba_tiled.bin");
        let expected = include_bytes!("../block_linear/16_16_16_rgba.bin");
        let actual = deswizzle_block_linear(16, 16, 16, input, BlockHeight::One, 4).unwrap();
        assert_eq!(expected, &actual[..]);
    }
}
