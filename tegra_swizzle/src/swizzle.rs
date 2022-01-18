//! Functions for swizzling and deswizzling.
use crate::{
    deswizzled_surface_size, swizzled_surface_size, width_in_gobs, BlockHeight, SwizzleError,
    GOB_HEIGHT_IN_BYTES, GOB_SIZE_IN_BYTES, GOB_WIDTH_IN_BYTES,
};

/// Swizzles the bytes from `source` using the block linear swizzling algorithm.
/// # Examples
/// Uncompressed formats like R8G8B8A8 can use the width and height in pixels.
/**
```rust
use tegra_swizzle::{BlockHeight, deswizzled_surface_size, swizzle::swizzle_block_linear};

let width = 512;
let height = 512;
# let size = deswizzled_surface_size(width, height, 1, 4);
# let input = vec![0u8; size];
let output = swizzle_block_linear(width, height, 1, &input, BlockHeight::Sixteen, 4);
```
 */
/// For compressed formats with multiple pixels in a block, divide the width and height by the block dimensions.
/**
```rust
# use tegra_swizzle::{BlockHeight, deswizzled_surface_size, swizzle::swizzle_block_linear};
// BC7 has 4x4 pixel blocks that each take up 16 bytes.
use tegra_swizzle::div_round_up;

let width = 512;
let height = 512;
# let size = deswizzled_surface_size(div_round_up(width, 4), div_round_up(height, 4), 1, 16);
# let input = vec![0u8; size];
let output = swizzle_block_linear(
    div_round_up(width, 4),
    div_round_up(height, 4),
    1,
    &input,
    BlockHeight::Sixteen,
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
        vec![0u8; swizzled_surface_size(width, height, depth, block_height, bytes_per_pixel)];

    let expected_size = deswizzled_surface_size(width, height, depth, bytes_per_pixel);
    if source.len() < expected_size {
        return Err(SwizzleError::NotEnoughData {
            actual_size: source.len(),
            expected_size,
        });
    }

    // TODO: Can we assume depth is block_depth?
    swizzle_inner(
        width,
        height,
        depth,
        source,
        &mut destination,
        block_height as usize,
        depth,
        bytes_per_pixel,
        false,
    );
    Ok(destination)
}

/// Deswizzles the bytes from `source` using the block linear swizzling algorithm.
/// # Examples
/// Uncompressed formats like R8G8B8A8 can use the width and height in pixels.
/**
```rust
use tegra_swizzle::{BlockHeight, swizzled_surface_size, swizzle::deswizzle_block_linear};

let width = 512;
let height = 512;
# let size = swizzled_surface_size(width, height, 1, BlockHeight::Sixteen, 4);
# let input = vec![0u8; size];
let output = deswizzle_block_linear(width, height, 1, &input, BlockHeight::Sixteen, 4);
```
 */
/// For compressed formats with multiple pixels in a block, divide the width and height by the block dimensions.
/**
```rust
# use tegra_swizzle::{BlockHeight, swizzled_surface_size, swizzle::deswizzle_block_linear};
// BC7 has 4x4 pixel blocks that each take up 16 bytes.
use tegra_swizzle::div_round_up;

let width = 512;
let height = 512;
# let size = swizzled_surface_size(div_round_up(width, 4), div_round_up(height, 4), 1, BlockHeight::Sixteen, 16);
# let input = vec![0u8; size];
let output = deswizzle_block_linear(
    div_round_up(width, 4),
    div_round_up(height, 4),
    1,
    &input,
    BlockHeight::Sixteen,
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
    let mut destination = vec![0u8; deswizzled_surface_size(width, height, depth, bytes_per_pixel)];

    let expected_size = swizzled_surface_size(width, height, depth, block_height, bytes_per_pixel);
    if source.len() < expected_size {
        return Err(SwizzleError::NotEnoughData {
            actual_size: source.len(),
            expected_size,
        });
    }

    // TODO: We can't assume depth is block_depth.
    swizzle_inner(
        width,
        height,
        depth,
        source,
        &mut destination,
        block_height as usize,
        depth,
        bytes_per_pixel,
        true,
    );
    Ok(destination)
}

pub(crate) fn swizzle_inner(
    width: usize,
    height: usize,
    depth: usize,
    source: &[u8],
    destination: &mut [u8],
    block_height: usize,
    block_depth: usize,
    bytes_per_pixel: usize,
    deswizzle: bool,
) {
    let image_width_in_gobs = width_in_gobs(width, bytes_per_pixel);

    // TODO: Is this correct?
    let slice_size = image_width_in_gobs * block_depth * GOB_SIZE_IN_BYTES;

    // Blocks are always one GOB wide.
    // TODO: Citation?
    let block_width = 1;
    let block_size_in_bytes = GOB_SIZE_IN_BYTES * block_width * block_height * block_depth;
    let block_height_in_bytes = GOB_HEIGHT_IN_BYTES * block_height;

    // Swizzling is defined as a mapping from byte coordinates x,y,z -> x',y',z'.
    // We step a GOB of bytes at a time to enable a tiled optimization approach.
    // GOBs always use the same swizzle patterns, so we can optimize swizzling complete 64x8 byte tiles.
    // The partially filled GOBs along the right and bottom edge use a slower per byte implementation.
    for z0 in 0..depth {
        let offset_z = gob_address_z(z0, block_height, block_depth, slice_size);

        // Step by a GOB of bytes in y.
        for y0 in (0..height).step_by(GOB_HEIGHT_IN_BYTES) {
            let offset_y = gob_address_y(
                y0,
                block_height_in_bytes,
                block_size_in_bytes,
                image_width_in_gobs,
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
                    if deswizzle {
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
                    swizzle_deswizzle_gob(
                        destination,
                        source,
                        x0,
                        y0,
                        z0,
                        width,
                        height,
                        bytes_per_pixel,
                        gob_address,
                        deswizzle,
                    );
                }
            }
        }
    }
}

fn swizzle_deswizzle_gob(
    destination: &mut [u8],
    source: &[u8],
    x0: usize,
    y0: usize,
    z0: usize,
    width: usize,
    height: usize,
    bytes_per_pixel: usize,
    gob_address: usize,
    deswizzle: bool,
) {
    for y in 0..GOB_HEIGHT_IN_BYTES {
        for x in 0..GOB_WIDTH_IN_BYTES {
            if y0 + y < height && x0 + x < width * bytes_per_pixel {
                let swizzled_offset = gob_address + gob_offset(x, y);
                let linear_offset = (z0 * width * height * bytes_per_pixel)
                    + ((y0 + y) * width * bytes_per_pixel)
                    + x0
                    + x;

                // Swap the addresses for swizzling vs deswizzling.
                if deswizzle {
                    destination[linear_offset] = source[swizzled_offset];
                } else {
                    destination[swizzled_offset] = source[linear_offset];
                }
            }
        }
    }
}

// TODO: Add additional 3D tests?
// Yuzu: https://github.com/yuzu-emu/yuzu/blob/c5ca8675c84ca73375cf3fe2ade257c8aa5c1239/src/video_core/textures/decoders.cpp#L46-L47
// Ryujinx: https://github.com/Ryujinx/Ryujinx/blob/1485780d90a554a9a71585ff1dd6e049b32b761e/Ryujinx.Graphics.Texture/BlockLinearLayout.cs#L146-L154
fn gob_address_z(z: usize, block_height: usize, block_depth: usize, slice_size: usize) -> usize {
    // Each "column" of blocks has block_depth many blocks.
    // A 16x16x16 RGBA8 3d texture has the following deswizzled GOB indices.
    // 0, 16, 1, 17, 2, 18, 3, 19, 4, 20, 5, 21, 6, 22, 7, 23, 8, 24,
    // 9, 25, 10, 26, 11, 27, 12, 28, 13, 29, 14, 30, 15, 31
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

// Code for offset_x and offset_y adapted from examples in the Tegra TRM page 1187.
fn gob_address_x(x: usize, block_size_in_bytes: usize) -> usize {
    let block_x = x / GOB_WIDTH_IN_BYTES;
    block_x * block_size_in_bytes
}

// Code taken from examples in Tegra TRM page 1188.
// Return the offset within the GOB for the byte at location (x, y).
fn gob_offset(x: usize, y: usize) -> usize {
    // TODO: Optimize this?
    // TODO: Describe the pattern here?
    ((x % 64) / 32) * 256 + ((y % 8) / 2) * 64 + ((x % 32) / 16) * 32 + (y % 2) * 16 + (x % 16)
}

// TODO: Investigate using macros to generate this code.

// An optimized version of the gob_offset for an entire GOB worth of bytes.
// The swizzled GOB is a contiguous region of 512 bytes.
// The deswizzled GOB is a 64x8 2D region of memory, so we need to account for the pitch.
fn deswizzle_complete_gob(dst: &mut [u8], src: &[u8], row_size_in_bytes: usize) {
    // Hard code each of the GOB_HEIGHT many rows.
    // This allows the compiler to optimize the copies with SIMD instructions.
    deswizzle_gob_row(dst, row_size_in_bytes * 0, src, 0);
    deswizzle_gob_row(dst, row_size_in_bytes * 1, src, 16);
    deswizzle_gob_row(dst, row_size_in_bytes * 2, src, 64);
    deswizzle_gob_row(dst, row_size_in_bytes * 3, src, 80);
    deswizzle_gob_row(dst, row_size_in_bytes * 4, src, 128);
    deswizzle_gob_row(dst, row_size_in_bytes * 5, src, 144);
    deswizzle_gob_row(dst, row_size_in_bytes * 6, src, 192);
    deswizzle_gob_row(dst, row_size_in_bytes * 7, src, 208);
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
    swizzle_gob_row(dst, 0, src, row_size_in_bytes * 0);
    swizzle_gob_row(dst, 16, src, row_size_in_bytes * 1);
    swizzle_gob_row(dst, 64, src, row_size_in_bytes * 2);
    swizzle_gob_row(dst, 80, src, row_size_in_bytes * 3);
    swizzle_gob_row(dst, 128, src, row_size_in_bytes * 4);
    swizzle_gob_row(dst, 144, src, row_size_in_bytes * 5);
    swizzle_gob_row(dst, 192, src, row_size_in_bytes * 6);
    swizzle_gob_row(dst, 208, src, row_size_in_bytes * 7);
}

fn swizzle_gob_row(dst: &mut [u8], dst_offset: usize, src: &[u8], src_offset: usize) {
    let dst = &mut dst[dst_offset..];
    let src = &src[src_offset..];
    dst[288..304].copy_from_slice(&src[48..64]);
    dst[256..272].copy_from_slice(&src[32..48]);
    dst[32..48].copy_from_slice(&src[16..32]);
    dst[0..16].copy_from_slice(&src[0..16]);
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
        // The swizzling algorithm should still handle these cases.
        let bytes_per_pixel = 12;

        let deswizzled_size = deswizzled_surface_size(width, height, 1, bytes_per_pixel);

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
        let swizzled = include_bytes!("../../swizzle_data/64_bc7_swizzled.bin");
        let deswizzled =
            deswizzle_block_linear(64 / 4, 64 / 4, 1, swizzled, BlockHeight::Two, 16).unwrap();

        let new_swizzled =
            swizzle_block_linear(64 / 4, 64 / 4, 1, &deswizzled, BlockHeight::Two, 16).unwrap();
        assert_eq!(swizzled, &new_swizzled[..]);
    }

    #[test]
    fn deswizzle_bc7_64_64() {
        let input = include_bytes!("../../swizzle_data/64_bc7_swizzled.bin");
        let expected = include_bytes!("../../swizzle_data/64_bc7_deswizzled.bin");
        let actual =
            deswizzle_block_linear(64 / 4, 64 / 4, 1, input, BlockHeight::Two, 16).unwrap();

        assert_eq!(expected, &actual[..]);
    }

    #[test]
    fn deswizzle_bc1_128_128() {
        let input = include_bytes!("../../swizzle_data/128_bc1_swizzled.bin");
        let expected = include_bytes!("../../swizzle_data/128_bc1_deswizzled.bin");
        let actual =
            deswizzle_block_linear(128 / 4, 128 / 4, 1, input, BlockHeight::Four, 8).unwrap();

        assert_eq!(expected, &actual[..]);
    }

    #[test]
    fn deswizzle_bc3_128_128() {
        let input = include_bytes!("../../swizzle_data/128_bc3_swizzled.bin");
        let expected = include_bytes!("../../swizzle_data/128_bc3_deswizzled.bin");
        let actual =
            deswizzle_block_linear(128 / 4, 128 / 4, 1, input, BlockHeight::Four, 16).unwrap();

        assert_eq!(expected, &actual[..]);
    }

    #[test]
    fn deswizzle_rgba_f32_128_128() {
        let input = include_bytes!("../../swizzle_data/128_rgbaf32_swizzled.bin");
        let expected = include_bytes!("../../swizzle_data/128_rgbaf32_deswizzled.bin");
        let actual = deswizzle_block_linear(128, 128, 1, input, BlockHeight::Sixteen, 16).unwrap();

        assert_eq!(expected, &actual[..]);
    }

    #[test]
    fn deswizzle_bc7_128_128() {
        let input = include_bytes!("../../swizzle_data/128_bc7_swizzled.bin");
        let expected = include_bytes!("../../swizzle_data/128_bc7_deswizzled.bin");
        let actual =
            deswizzle_block_linear(128 / 4, 128 / 4, 1, input, BlockHeight::Four, 16).unwrap();

        assert_eq!(expected, &actual[..]);
    }

    #[test]
    fn deswizzle_bc7_256_256() {
        let input = include_bytes!("../../swizzle_data/256_bc7_swizzled.bin");
        let expected = include_bytes!("../../swizzle_data/256_bc7_deswizzled.bin");
        let actual =
            deswizzle_block_linear(256 / 4, 256 / 4, 1, input, BlockHeight::Eight, 16).unwrap();

        assert_eq!(expected, &actual[..]);
    }

    #[test]
    fn deswizzle_bc7_320_320() {
        let input = include_bytes!("../../swizzle_data/320_bc7_swizzled.bin");
        let expected = include_bytes!("../../swizzle_data/320_bc7_deswizzled.bin");
        let actual =
            deswizzle_block_linear(320 / 4, 320 / 4, 1, input, BlockHeight::Eight, 16).unwrap();

        assert_eq!(expected, &actual[..]);
    }

    #[test]
    fn deswizzle_bc7_512_512() {
        let input = include_bytes!("../../swizzle_data/512_bc7_swizzled.bin");
        let expected = include_bytes!("../../swizzle_data/512_bc7_deswizzled.bin");
        let actual =
            deswizzle_block_linear(512 / 4, 512 / 4, 1, input, BlockHeight::Sixteen, 16).unwrap();

        assert_eq!(expected, &actual[..]);
    }

    #[test]
    fn deswizzle_bc7_1024_1024() {
        let input = include_bytes!("../../swizzle_data/1024_bc7_swizzled.bin");
        let expected = include_bytes!("../../swizzle_data/1024_bc7_deswizzled.bin");
        let actual =
            deswizzle_block_linear(1024 / 4, 1024 / 4, 1, input, BlockHeight::Sixteen, 16).unwrap();

        assert_eq!(expected, &actual[..]);
    }

    #[test]
    fn deswizzle_rgba_16_16_16() {
        let input = include_bytes!("../../swizzle_data/16_16_16_rgba_swizzled.bin");
        let expected = include_bytes!("../../swizzle_data/16_16_16_rgba_deswizzled.bin");
        let actual = deswizzle_block_linear(16, 16, 16, input, BlockHeight::One, 4).unwrap();
        assert_eq!(expected, &actual[..]);
    }
}
