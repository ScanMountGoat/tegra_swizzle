//! The [swizzle_block_linear] and [deswizzle_block_linear] functions implement the texture swizzling algorithm for the Tegra X1's block linear format.
//!
//! Block linear arranges bytes of a texture surface into a 2D grid of blocks.
//! Groups of 512 bytes form GOBs ("group of bytes") where each GOB is 64x8 bytes.
//! The `block_height` parameter determines how many GOBs stack vertically to form a block.
//!
//! Blocks are arranged linearly in row-major order. Each block has a width of 1 GOB and a height of `block_height` many GOBs.
//!
//! Pixels or 4x4 pixel tiles for BCN compressed formats are arranged horizontally to form a row of `width_in_pixels * bytes_per_pixel` many bytes
//! or `width_in_pixels / 4 * bytes_per_tile` many bytes.
//! The surface width is rounded up to the width in blocks or 64 bytes (one GOB).
//!
//! The height of the surface is `height_in_pixels` many bytes for most formats and
//! `height_in_pixels / 4` many bytes for BCN compressed formats since the tiles are arranged horizontally within the row.
//! The surface height is rounded up to the height in blocks or `block_height * 8` bytes.

// #![no_std]
// TODO: We don't need std since the core crate can provide the necessary memcpy operation.

/// The width in bytes for a single Group of Bytes (GOB).
const GOB_WIDTH: usize = 64;

/// The height in bytes for a single Group of Bytes (GOB).
const GOB_HEIGHT: usize = 8;

/// The size in bytes for a single Group of Bytes (GOB).
const GOB_SIZE: usize = GOB_WIDTH * GOB_HEIGHT;

// Block height can only have certain values based on the Tegra TRM page 1189 table 79.

/// An enumeration of supported block heights.
///
/// Texture file formats differ in how they encode the block height parameter.
/// Some formats may encode block height using log2, so a block height of 8 would be encoded as 3.
/// Other formats may infer the block height based on texture dimensions and other factors.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub enum BlockHeight {
    One = 1,
    Two = 2,
    Four = 4,
    Eight = 8,
    Sixteen = 16,
    ThirtyTwo = 32,
}

/// Errors than can occur while swizzling or deswizzling.
#[derive(Debug)]
pub enum SwizzleError {
    /// The source data does not contain enough bytes.
    /// The input length should be at least [swizzled_surface_size] many bytes for deswizzling
    /// and at least [deswizzled_surface_size] many bytes for swizzling.
    NotEnoughData {
        expected_size: usize,
        actual_size: usize,
    },
}

impl BlockHeight {
    // TODO: Make this public and add a proper error type.
    fn from_int(v: usize) -> Self {
        match v {
            1 => BlockHeight::One,
            2 => BlockHeight::Two,
            4 => BlockHeight::Four,
            8 => BlockHeight::Eight,
            16 => BlockHeight::Sixteen,
            32 => BlockHeight::ThirtyTwo,
            _ => panic!("Unsupported block height"),
        }
    }
}

// Code taken from examples in Tegra TRM page 1187.
// Return the starting address of the GOB containing the pixel at location (x, y).
fn gob_address(
    x: usize,
    y: usize,
    z: usize,
    block_height: usize,
    block_depth: usize,
    image_width_in_gobs: usize,
    height_in_blocks: usize,
) -> usize {
    // TODO: Benchmark moving the x,y,z components into their corresponding loops.
    let block_height_in_bytes = GOB_HEIGHT * block_height;

    // Blocks are always one GOB wide.
    // TODO: Citation?
    let block_width = 1;
    let block_size_in_bytes = GOB_SIZE * block_width * block_height * block_depth;

    let block_y = y / block_height_in_bytes;

    let block_x = x / GOB_WIDTH;

    // GOBs are stacked vertically within a block.
    // The following example shows two blocks with block height of two.
    // Setting the hex editor width to image_width_in_gobs * 64 makes the structure clearer.
    // Block0 consists GOB0 and GOB1, and Block1 consists of GOB2 and GOB3.
    // GOB0       GOB2
    // GOB1       GOB3
    let block_inner_row = y % block_height_in_bytes / GOB_HEIGHT;

    // This is the gob layout for a texture with
    // width in gobs of 1, block height = 1, and depth = 16.
    // GOB0    GOB16
    // GOB1    GOB17
    // ....    ...
    // GOB15   GOB32
    // Each "column" of blocks has block_depth many blocks.
    // TODO: There's currently only a single 16x16x16 RGBA example, so this is hardcoded for now.
    // The math seems to be based on block_depth of 16 bytes?
    // TODO: Where does the number 512 come from?
    let offset_z = z * 512;

    let offset_y = block_y * block_size_in_bytes * image_width_in_gobs + block_inner_row * GOB_SIZE;

    let offset_x = block_x * block_size_in_bytes;

    // Linear (row-major) block indexing.
    offset_z + offset_y + offset_x
}

// Code taken from examples in Tegra TRM page 1188.
// Return the offset within the GOB for the byte at location (x, y).
fn gob_offset(x: usize, y: usize) -> usize {
    // TODO: Optimize this?
    // TODO: Describe the pattern here?
    ((x % 64) / 32) * 256 + ((y % 8) / 2) * 64 + ((x % 32) / 16) * 32 + (y % 2) * 16 + (x % 16)
}

// Given pixel coordinates (x, y), find the offset in the swizzled image data.
// This can be used for swizzling and deswizzling operations.
fn swizzled_address(
    x: usize,
    y: usize,
    z: usize,
    block_height: usize,
    block_depth: usize,
    image_width_in_gobs: usize,
    height_in_blocks: usize,
    bytes_per_pixel: usize,
    i: usize, // HACK: offset within a pixel?
) -> usize {
    let gob_address = gob_address(
        x * bytes_per_pixel + i,
        y,
        z,
        block_height,
        block_depth,
        image_width_in_gobs,
        height_in_blocks,
    );

    // Multiply by bytes_per_pixel since this function expects byte coordinates.
    // We assume 1 byte per row, so y is left unchanged.
    let gob_offset = gob_offset(x * bytes_per_pixel + i, y);

    gob_address + gob_offset
}

/// Calculates the size in bytes for the swizzled data for the given dimensions for the block linear format.
/// The result of [swizzled_surface_size] will always be at least as large as [deswizzled_surface_size].
pub const fn swizzled_surface_size(
    width: usize,
    height: usize,
    depth: usize,
    block_height: BlockHeight,
    bytes_per_pixel: usize,
) -> usize {
    let width_in_gobs = width_in_gobs(width, bytes_per_pixel);
    let height_in_blocks = height_in_blocks(height, block_height as usize);
    width_in_gobs * height_in_blocks * block_height as usize * GOB_SIZE * depth
}

const fn height_in_blocks(height: usize, block_height: usize) -> usize {
    div_round_up(height, block_height * GOB_HEIGHT)
}

/// Calculates the size in bytes for the deswizzled data for the given dimensions.
/// Compare with [swizzled_surface_size].
pub const fn deswizzled_surface_size(
    width: usize,
    height: usize,
    depth: usize,
    bytes_per_pixel: usize,
) -> usize {
    width * height * depth * bytes_per_pixel
}

/// Gets the height of each block in GOBs for the specified `height`.
/// For formats that compress multiple pixels into a single tile, divide the height in pixels by the tile height.
/// # Examples
///
/// Non compressed formats can typically just use the height in pixels.
/**
```rust
# use nutexb_swizzle::BlockHeight;
let height_in_pixels = 512;
assert_eq!(BlockHeight::Sixteen, nutexb_swizzle::block_height(height_in_pixels));
```
*/
/// BCN formats work in 4x4 tiles instead of pixels, so divide the height by 4 since each tile is 4 pixels high.
/**
```rust
# use nutexb_swizzle::BlockHeight;
let height_in_pixels = 512;
assert_eq!(BlockHeight::Sixteen, nutexb_swizzle::block_height(height_in_pixels / 4));
```
*/
pub fn block_height(height: usize) -> BlockHeight {
    let block_height = div_round_up(height, 8);

    // TODO: Is it correct to find the closest power of two?
    // TODO: This is only valid for nutexb, so it likely shouldn't be part of this API.
    match block_height {
        0..=1 => BlockHeight::One,
        2 => BlockHeight::Two,
        3..=4 => BlockHeight::Four,
        5..=8 => BlockHeight::Eight, // TODO: This doesn't work for 320x320 BC7 mip0?
        // TODO: The TRM mentions 32 also works?
        _ => BlockHeight::Sixteen,
    }
}

const fn div_round_up(x: usize, d: usize) -> usize {
    (x + d - 1) / d
}

const fn width_in_gobs(width: usize, bytes_per_pixel: usize) -> usize {
    div_round_up(width * bytes_per_pixel, GOB_WIDTH)
}

/// Swizzles the bytes from `source` using the block linear swizzling algorithm.
/// # Examples
/// Uncompressed formats like R8G8B8A8 can use the width and height in pixels.
/**
```rust
use nutexb_swizzle::{BlockHeight, deswizzled_surface_size, swizzle_block_linear};

let width = 512;
let height = 512;
# let size = deswizzled_surface_size(width, height, 1, 4);
# let input = vec![0u8; size];
let output = swizzle_block_linear(width, height, 1, &input, BlockHeight::Sixteen, 4);
```
 */
/// For compressed formats with multiple pixels in a block or tile, divide the width and height by the tile dimensions.
/**
```rust
# use nutexb_swizzle::{BlockHeight, deswizzled_surface_size, swizzle_block_linear};
// BC7 has 4x4 pixel tiles that each take up 16 bytes.
let width = 512;
let height = 512;
# let size = deswizzled_surface_size(width / 4, height / 4, 1, 16);
# let input = vec![0u8; size];
let output = swizzle_block_linear(width / 4, height / 4, 1, &input, BlockHeight::Sixteen, 16);
```
 */
/// # Panics
/// Panics on out of bounds accesses for `source` or `destination`.
/// `source` is expected to have at least [deswizzled_surface_size] many bytes.
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

fn swizzle_inner(
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
    // TODO: It's possible to write this in a way to generate optimized SIMD code instead of integer arithmetic and memcpy calls.
    // The swizzling always moves groups of 16 bytes, which can be done using copy_from_slice with fixed ranges.
    // This will require some careful testing to make sure the approach works for swizzling as well as deswizzling.
    // The input slice size will likely require some alignment for this to work effectively.

    // This naive solution can serve as a reference for later implementations.
    let image_width_in_gobs = width_in_gobs(width, bytes_per_pixel);
    let height_in_blocks = height_in_blocks(height, block_height);

    for z in 0..depth {
        for y in 0..height {
            for x in 0..width {
                for i in 0..bytes_per_pixel {
                    let swizzled_offset = swizzled_address(
                        x,
                        y,
                        z,
                        block_height,
                        block_depth,
                        image_width_in_gobs,
                        height_in_blocks,
                        bytes_per_pixel,
                        i,
                    );
                    let linear_offset = (z * width * height + y * width + x) * bytes_per_pixel + i;

                    // TODO: Does this condition optimize out since we specify it at compile time?
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
}

/// Deswizzles the bytes from `source` using the block linear swizzling algorithm.
/// # Examples
/// Uncompressed formats like R8G8B8A8 can use the width and height in pixels.
/**
```rust
use nutexb_swizzle::{BlockHeight, swizzled_surface_size, deswizzle_block_linear};

let width = 512;
let height = 512;
# let size = swizzled_surface_size(width, height, 1, BlockHeight::Sixteen, 4);
# let input = vec![0u8; size];
let output = deswizzle_block_linear(width, height, 1, &input, BlockHeight::Sixteen, 4);
```
 */
/// For compressed formats with multiple pixels in a block or tile, divide the width and height by the tile dimensions.
/**
```rust
# use nutexb_swizzle::{BlockHeight, swizzled_surface_size, deswizzle_block_linear};
// BC7 has 4x4 pixel tiles that each take up 16 bytes.
let width = 512;
let height = 512;
# let size = swizzled_surface_size(width / 4, height / 4, 1, BlockHeight::Sixteen, 16);
# let input = vec![0u8; size];
let output = deswizzle_block_linear(width / 4, height / 4, 1, &input, BlockHeight::Sixteen, 16);
```
 */
/// # Panics
/// Panics on out of bounds accesses during swizzling.
/// `source` is expected to have at least as many bytes as the result of [swizzled_surface_size].
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
        true,
    );
    Ok(destination)
}

pub mod ffi {
    use crate::BlockHeight;

    /// Swizzles the bytes from `source` into `destination` using the block linear swizzling algorithm.
    /// See the safe alternative [swizzle_block_linear](super::swizzle_block_linear).
    /// # Safety
    /// `source` and `source_len` should refer to an array with at least as many bytes as the result of [deswizzled_surface_size].
    /// Similarly, `destination` and `destination_len` should refer to an array with at least as many bytes as as the result of [swizzled_surface_size].
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

        super::swizzle_inner(
            width,
            height,
            depth,
            source,
            destination,
            // TODO: Check that block_height is a valid value?
            BlockHeight::from_int(block_height) as usize,
            depth,
            bytes_per_pixel,
            false,
        )
    }

    /// Deswizzles the bytes from `source` into `destination` using the block linear swizzling algorithm.
    /// See the safe alternative [deswizzle_block_linear](super::deswizzle_block_linear).
    /// # Safety
    /// `source` and `source_len` should refer to an array with at least as many bytes as the result of [swizzled_surface_size].
    /// Similarly, `destination` and `destination_len` should refer to an array with at least as many bytes as as the result of [deswizzled_surface_size].
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

        super::swizzle_inner(
            width,
            height,
            depth,
            source,
            destination,
            BlockHeight::from_int(block_height) as usize,
            depth,
            bytes_per_pixel,
            true,
        )
    }

    /// See [swizzled_surface_size](super::swizzled_surface_size).
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
            BlockHeight::from_int(block_height),
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

    /// See [block_height](super::block_height).
    #[no_mangle]
    pub extern "C" fn block_height(height: usize) -> usize {
        super::block_height(height) as usize
    }
}

#[cfg(test)]
mod tests {
    use rand::{rngs::StdRng, Rng, SeedableRng};

    use super::*;

    #[test]
    fn swizzle_deswizzle_bytes_per_pixel() {
        let width = 312;
        let height = 575;
        let block_height = BlockHeight::Eight;

        // Test a value that isn't 4, 8, or 16.
        // Non standard values won't show up in practice,
        // but the algorithm can still handle these cases in theory.
        let bytes_per_pixel = 12;

        let deswizzled_size = deswizzled_surface_size(width, height, 1, bytes_per_pixel);

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
        let err = swizzle_block_linear(32, 32, 1, &[], BlockHeight::Sixteen, 4);
        assert!(matches!(
            err,
            Err(SwizzleError::NotEnoughData {
                actual_size: 0,
                expected_size: 4096
            })
        ));
    }

    #[test]
    fn deswizzle_empty() {
        let err = deswizzle_block_linear(32, 32, 1, &[], BlockHeight::Sixteen, 4);
        assert!(matches!(
            err,
            Err(SwizzleError::NotEnoughData {
                actual_size: 0,
                expected_size: 16384
            })
        ));
    }

    #[test]
    fn deswizzle_bc7_64_64() {
        let input = include_bytes!("../../swizzle_data/64_bc7_linear.bin");
        let expected = include_bytes!("../../swizzle_data/64_bc7_linear_deswizzle.bin");
        let actual =
            deswizzle_block_linear(64 / 4, 64 / 4, 1, input, BlockHeight::Two, 16).unwrap();

        assert_eq!(expected, &actual[..]);
    }

    #[test]
    fn deswizzle_bc1_128_128() {
        let input = include_bytes!("../../swizzle_data/128_bc1_linear.bin");
        let expected = include_bytes!("../../swizzle_data/128_bc1_linear_deswizzle.bin");
        let actual =
            deswizzle_block_linear(128 / 4, 128 / 4, 1, input, BlockHeight::Four, 8).unwrap();

        assert_eq!(expected, &actual[..]);
    }

    #[test]
    fn deswizzle_bc3_128_128() {
        let input = include_bytes!("../../swizzle_data/128_bc3_linear.bin");
        let expected = include_bytes!("../../swizzle_data/128_bc3_linear_deswizzle.bin");
        let actual =
            deswizzle_block_linear(128 / 4, 128 / 4, 1, input, BlockHeight::Four, 16).unwrap();

        assert_eq!(expected, &actual[..]);
    }

    #[test]
    fn deswizzle_rgba_f32_128_128() {
        let input = include_bytes!("../../swizzle_data/128_rgbaf32_linear.bin");
        let expected = include_bytes!("../../swizzle_data/128_rgbaf32_linear_deswizzle.bin");
        let actual = deswizzle_block_linear(128, 128, 1, input, BlockHeight::Sixteen, 16).unwrap();

        assert_eq!(expected, &actual[..]);
    }

    #[test]
    fn deswizzle_bc7_128_128() {
        let input = include_bytes!("../../swizzle_data/128_bc7_linear.bin");
        let expected = include_bytes!("../../swizzle_data/128_bc7_linear_deswizzle.bin");
        let actual =
            deswizzle_block_linear(128 / 4, 128 / 4, 1, input, BlockHeight::Four, 16).unwrap();

        assert_eq!(expected, &actual[..]);
    }

    #[test]
    fn deswizzle_bc7_256_256() {
        let input = include_bytes!("../../swizzle_data/256_bc7_linear.bin");
        let expected = include_bytes!("../../swizzle_data/256_bc7_linear_deswizzle.bin");
        let actual =
            deswizzle_block_linear(256 / 4, 256 / 4, 1, input, BlockHeight::Eight, 16).unwrap();

        assert_eq!(expected, &actual[..]);
    }

    #[test]
    fn deswizzle_bc7_320_320() {
        let input = include_bytes!("../../swizzle_data/320_bc7_linear.bin");
        let expected = include_bytes!("../../swizzle_data/320_bc7_linear_deswizzle.bin");
        let actual =
            deswizzle_block_linear(320 / 4, 320 / 4, 1, input, BlockHeight::Eight, 16).unwrap();

        assert_eq!(expected, &actual[..]);
    }

    #[test]
    fn deswizzle_bc7_512_512() {
        let input = include_bytes!("../../swizzle_data/512_bc7_linear.bin");
        let expected = include_bytes!("../../swizzle_data/512_bc7_linear_deswizzle.bin");
        let actual =
            deswizzle_block_linear(512 / 4, 512 / 4, 1, input, BlockHeight::Sixteen, 16).unwrap();

        assert_eq!(expected, &actual[..]);
    }

    #[test]
    fn deswizzle_bc7_1024_1024() {
        let input = include_bytes!("../../swizzle_data/1024_bc7_linear.bin");
        let expected = include_bytes!("../../swizzle_data/1024_bc7_linear_deswizzle.bin");
        let actual =
            deswizzle_block_linear(1024 / 4, 1024 / 4, 1, input, BlockHeight::Sixteen, 16).unwrap();

        assert_eq!(expected, &actual[..]);
    }

    #[test]
    fn deswizzle_rgba_16_16_16() {
        let input = include_bytes!("../../swizzle_data/16_16_16_rgba_linear.bin");
        let expected = include_bytes!("../../swizzle_data/16_16_16_rgba_linear_deswizzle.bin");
        let actual = deswizzle_block_linear(16, 16, 16, input, BlockHeight::One, 4).unwrap();
        assert_eq!(expected, &actual[..]);
    }

    #[test]
    fn width_in_gobs_block16() {
        assert_eq!(20, width_in_gobs(320 / 4, 16));
    }

    #[test]
    fn block_heights() {
        assert_eq!(BlockHeight::Eight, block_height(64));

        // TODO: This should be 8 for the first mip level?
        assert_eq!(BlockHeight::Sixteen, block_height(320 / 4));

        // BCN Tiles.
        assert_eq!(BlockHeight::Sixteen, block_height(768 / 4));
        assert_eq!(BlockHeight::Sixteen, block_height(384 / 4));
        assert_eq!(BlockHeight::Sixteen, block_height(384 / 4));
        assert_eq!(BlockHeight::Four, block_height(80 / 4));
    }

    #[test]
    fn surface_sizes_block4() {
        assert_eq!(
            1048576,
            swizzled_surface_size(512, 512, 1, block_height(512), 4)
        );
    }

    #[test]
    fn surface_sizes_3d() {
        assert_eq!(
            16384,
            swizzled_surface_size(16, 16, 16, BlockHeight::Two, 4)
        );
    }

    #[test]
    fn surface_sizes_block16() {
        assert_eq!(
            163840,
            swizzled_surface_size(320 / 4, 320 / 4, 1, block_height(320 / 4), 16)
        );
        assert_eq!(
            40960,
            swizzled_surface_size(160 / 4, 160 / 4, 1, block_height(160 / 4), 16)
        );
        assert_eq!(
            1024,
            swizzled_surface_size(32 / 4, 32 / 4, 1, block_height(32 / 4), 16)
        );
    }
}
