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
mod swizzle;

pub mod imagedata;

// TODO: Separate module for swizzle?

// Avoid making this module public to prevent people importing it accidentally.
mod ffi;

pub use blockheight::*;
pub use swizzle::*;

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

#[cfg(test)]
mod tests {
    use super::*;

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
