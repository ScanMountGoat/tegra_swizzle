//! # tegra_swizzle
//! tegra_swizzle is an unofficial CPU implementation for Tegra X1 surface swizzling.
//!
//! # Getting Started
//! The following example demonstrates deswizzling mipmaps for a BC7 compressed 2D surface.
//! BC7 has 4x4 pixel blocks that each take up 16 bytes.
//! For uncompressed formats like R8G8B8A8, the [div_round_up] calls are unnecessary.
/*!
```rust no_run
use tegra_swizzle::{
    block_height_mip0, deswizzle_block_linear, div_round_up, mip_block_height,
    swizzled_surface_size
};
# fn main() -> Result<(), tegra_swizzle::SwizzleError> {
# let image_data = vec![0u8; 4];
# let height = 300;
# let width = 128;
# let mipmap_count = 5;
// Infer the block height if the surface doesn't specify one.
let block_height_mip0 = block_height_mip0(div_round_up(height, 4));
// It's common for all mipmaps to be stored in a contiguous region on disk.
// We'll simply update the starting offset for each level.
let mut offset = 0;
for mip in 0..mipmap_count {
    let mip_width = std::cmp::max(div_round_up(width >> mip, 4), 1);
    let mip_height = std::cmp::max(div_round_up(height >> mip, 4), 1);
    // The block height will likely change for each mip level.
    let mip_block_height = mip_block_height(mip_height, block_height_mip0);
    let deswizzled_mipmap = deswizzle_block_linear(
        mip_width,
        mip_height,
        1,
        &image_data[offset..],
        mip_block_height,
        16,
    )?;
    offset += swizzled_surface_size(mip_width, mip_height, 1, mip_block_height, 16);
}
# Ok(())
# }
```
*/
//! # Block Linear Swizzling
//! The [swizzle_block_linear] and [deswizzle_block_linear] functions
//! implement safe and efficient swizzling for the Tegra X1's block linear format.
//!
//! Block linear arranges bytes of a texture surface into a 2D grid of blocks
//! where blocks are arranged linearly in row-major order.
//! The swizzled surface size is padded to integral dimensions in blocks, so
//! swizzled surfaces may be larger than the corresponding data in row-major order.
//!
//! Groups of 512 bytes form GOBs ("group of bytes") where each GOB is 64x8 bytes.
//! The `block_height` parameter determines how many GOBs stack vertically to form a block.
//!
//! # Limitations
//! 2D surfaces are fully supported with minimal support for 3D surfaces.
//! Array textures such as cube maps may require additional alignment to work properly.
//! Depth values other than 1 are not guaranteed to work properly at this time.
//! These limitations should hopefully be fixed in a future release.
mod arrays;
mod blockdepth;
mod blockheight;

pub mod imagedata;

// TODO: Separate module for swizzle?

// Avoid making this module public to prevent people importing it accidentally.
mod ffi;

pub use blockheight::*;

const GOB_WIDTH_IN_BYTES: usize = 64;
const GOB_HEIGHT_IN_BYTES: usize = 8;
const GOB_SIZE_IN_BYTES: usize = GOB_WIDTH_IN_BYTES * GOB_HEIGHT_IN_BYTES;

// Block height can only have certain values based on the Tegra TRM page 1189 table 79.

/// An enumeration of supported block heights.
///
/// Texture file formats differ in how they encode the block height parameter.
/// Some formats may encode block height using log2, so a block height of 8 would be encoded as 3.
/// For formats that do not explicitly store block height, see [block_height_mip0].
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

impl std::fmt::Display for SwizzleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SwizzleError::NotEnoughData {
                expected_size,
                actual_size,
            } => write!(
                f,
                "Not enough data. Expected {} bytes but found {} bytes.",
                expected_size, actual_size
            ),
        }
    }
}

impl std::error::Error for SwizzleError {}

impl BlockHeight {
    /// Attempts to construct a block height from `value`.
    /// Returns [None] if `value` is not a supported block height.
    /// # Examples
    /**
    ```rust
    use tegra_swizzle::BlockHeight;

    assert_eq!(Some(BlockHeight::Eight), BlockHeight::new(8));
    assert_eq!(None, BlockHeight::new(5));
    ```
    */
    pub fn new(value: usize) -> Option<Self> {
        match value {
            1 => Some(BlockHeight::One),
            2 => Some(BlockHeight::Two),
            4 => Some(BlockHeight::Four),
            8 => Some(BlockHeight::Eight),
            16 => Some(BlockHeight::Sixteen),
            32 => Some(BlockHeight::ThirtyTwo),
            _ => None,
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

/// Calculates the size in bytes for the swizzled data for the given dimensions for the block linear format.
/// The result of [swizzled_surface_size] will always be at least as large as [deswizzled_surface_size]
/// for the same surface parameters.
/// # Examples
/// Uncompressed formats like R8G8B8A8 can use the width and height in pixels.
/**
```rust
use tegra_swizzle::{BlockHeight, swizzled_surface_size};

let width = 256;
let height = 256;
assert_eq!(262144, swizzled_surface_size(width, height, 1, BlockHeight::Sixteen, 4));
```
 */
/// For compressed formats with multiple pixels in a block, divide the width and height by the block dimensions.
/**
```rust
# use tegra_swizzle::{BlockHeight, swizzled_surface_size};
// BC7 has 4x4 pixel blocks that each take up 16 bytes.
use tegra_swizzle::div_round_up;

let width = 256;
let height = 256;
assert_eq!(
    131072,
    swizzled_surface_size(div_round_up(width, 4), div_round_up(height, 4), 1, BlockHeight::Sixteen, 16)
);
```
 */
pub const fn swizzled_surface_size(
    width: usize,
    height: usize,
    depth: usize,
    block_height: BlockHeight,
    bytes_per_pixel: usize,
) -> usize {
    let width_in_gobs = width_in_gobs(width, bytes_per_pixel);
    let height_in_blocks = height_in_blocks(height, block_height as usize);
    width_in_gobs * height_in_blocks * block_height as usize * GOB_SIZE_IN_BYTES * depth
}

const fn height_in_blocks(height: usize, block_height: usize) -> usize {
    div_round_up(height, block_height * GOB_HEIGHT_IN_BYTES)
}

/// Calculates the size in bytes for the deswizzled data for the given dimensions.
/// Compare with [swizzled_surface_size].
/// # Examples
/// Uncompressed formats like R8G8B8A8 can use the width and height in pixels.
/**
```rust
use tegra_swizzle::{BlockHeight, deswizzled_surface_size};

let width = 256;
let height = 256;
assert_eq!(262144, deswizzled_surface_size(width, height, 1, 4));
```
 */
/// For compressed formats with multiple pixels in a block, divide the width and height by the block dimensions.
/**
```rust
# use tegra_swizzle::{BlockHeight, deswizzled_surface_size};
// BC7 has 4x4 pixel blocks that each take up 16 bytes.
use tegra_swizzle::div_round_up;

let width = 256;
let height = 256;
assert_eq!(
    65536,
    deswizzled_surface_size(div_round_up(width, 4), div_round_up(height, 4), 1, 16)
);
```
 */
pub const fn deswizzled_surface_size(
    width: usize,
    height: usize,
    depth: usize,
    bytes_per_pixel: usize,
) -> usize {
    width * height * depth * bytes_per_pixel
}

/// Calculates the division of `x` by `d` but rounds up rather than truncating.
///
/// # Examples
/// Use this function when calculating dimensions for block compressed formats like BC7.
/**
```rust
# use tegra_swizzle::div_round_up;
assert_eq!(2, div_round_up(8, 4));
assert_eq!(3, div_round_up(10, 4));
```
 */
/// Uncompressed formats are equivalent to 1x1 pixel blocks.
/// The call to [div_round_up] can simply be ommitted in these cases.
/**
```rust
# use tegra_swizzle::div_round_up;
let n = 10;
assert_eq!(n, div_round_up(n, 1));
```
 */
#[inline]
pub const fn div_round_up(x: usize, d: usize) -> usize {
    (x + d - 1) / d
}

fn round_up(x: usize, n: usize) -> usize {
    ((x + n - 1) / n) * n
}

const fn width_in_gobs(width: usize, bytes_per_pixel: usize) -> usize {
    div_round_up(width * bytes_per_pixel, GOB_WIDTH_IN_BYTES)
}

/// Swizzles the bytes from `source` using the block linear swizzling algorithm.
/// # Examples
/// Uncompressed formats like R8G8B8A8 can use the width and height in pixels.
/**
```rust
use tegra_swizzle::{BlockHeight, deswizzled_surface_size, swizzle_block_linear};

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
# use tegra_swizzle::{BlockHeight, deswizzled_surface_size, swizzle_block_linear};
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
use tegra_swizzle::{BlockHeight, swizzled_surface_size, deswizzle_block_linear};

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
# use tegra_swizzle::{BlockHeight, swizzled_surface_size, deswizzle_block_linear};
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
        // Non standard values won't show up in practice.
        // The swizzling algorithm should still handle these cases.
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

    #[test]
    fn width_in_gobs_block16() {
        assert_eq!(20, width_in_gobs(320 / 4, 16));
    }

    #[test]
    fn deswizzled_surface_sizes() {
        assert_eq!(3145728, deswizzled_surface_size(512, 512, 3, 4));
    }

    #[test]
    fn surface_sizes_block4() {
        assert_eq!(
            1048576,
            swizzled_surface_size(512, 512, 1, BlockHeight::Sixteen, 4)
        );
    }

    #[test]
    fn surface_sizes_3d() {
        assert_eq!(
            16384,
            swizzled_surface_size(16, 16, 16, BlockHeight::One, 4)
        );
    }

    #[test]
    fn surface_sizes_block16() {
        assert_eq!(
            163840,
            swizzled_surface_size(320 / 4, 320 / 4, 1, BlockHeight::Sixteen, 16)
        );
        assert_eq!(
            40960,
            swizzled_surface_size(160 / 4, 160 / 4, 1, BlockHeight::Four, 16)
        );
        assert_eq!(
            1024,
            swizzled_surface_size(32 / 4, 32 / 4, 1, BlockHeight::One, 16)
        );
    }
}
