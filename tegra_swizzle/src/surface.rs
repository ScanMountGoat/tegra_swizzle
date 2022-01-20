//! Functions for working with surfaces stored in a combined buffer for all arrays and mipmaps.
//!
//! It's common for texture surfaces to be represented
//! as a single allocated region of memory that contains all array layers and mipmaps.
//! This also applies to the swizzled surfaces used for textures on the Tegra X1.
//!
//! Use [deswizzle_surface] for reading swizzled surfaces into a single deswizzled `Vec<u8>`.
//! This output can be used as is for creating DDS files.
//! Modern graphics APIs like Vulkan support this dense layout for initializing all
//! array layers and mipmaps for a texture in a single API call.
//!
//! Use [swizzle_surface] for writing a swizzled surface from a combined buffer like the result of [deswizzle_surface] or a DDS file.
//! This is the image data layout expected for some texture file formats.
//!
//! # Examples
//! Array layers and mipmaps are ordered by layer and then mipmap.
//! A surface with `L` layers and `M` mipmaps would have the following layout.
/*!
```no_compile
Layer 0 Mip 0
Layer 0 Mip 1
...
Layer 0 Mip M,
Layer 1 Mip 0,
Layer 1 Mip 1
...
Layer L Mip M
```
*/
//! The convention is for the non swizzled layout to be tightly packed.
//! Swizzled surfaces add additional padding and alignment between layers and mipmaps.
use std::{cmp::max, num::NonZeroUsize};

use crate::{
    arrays::align_layer_size, deswizzled_surface_size, div_round_up, mip_block_height,
    swizzle::deswizzle_block_linear, swizzle::swizzle_block_linear, swizzled_surface_size,
    BlockHeight, SwizzleError,
};

/// The dimensions of a compressed block. Compressed block sizes are usually 4x4.
pub struct BlockDim {
    /// The width of the block in pixels.
    pub width: NonZeroUsize,
    /// The height of the block in pixels.
    pub height: NonZeroUsize,
    /// The depth of the block in pixels.
    pub depth: NonZeroUsize,
}

impl BlockDim {
    /// A 1x1x1 block for formats that do not use block compression like R8G8B8A8.
    pub fn uncompressed() -> Self {
        BlockDim {
            width: NonZeroUsize::new(1).unwrap(),
            height: NonZeroUsize::new(1).unwrap(),
            depth: NonZeroUsize::new(1).unwrap(),
        }
    }

    /// A 4x4x1 compressed block. This includes any of the BCN formats like BC1, BC3, or BC7.
    /// This also includes DXT1, DXT3, and DXT5.
    pub fn block_4x4() -> Self {
        BlockDim {
            width: NonZeroUsize::new(4).unwrap(),
            height: NonZeroUsize::new(4).unwrap(),
            depth: NonZeroUsize::new(1).unwrap(),
        }
    }
}

// TODO: Create an inner function to reduce duplicate code?

/// Swizzles all the array layers and mipmaps in `source` using the block linear algorithm
/// to a combined vector with appropriate mipmap and array alignment.
///
/// Set `block_height_mip0` to [None] to infer the block height from the specified dimensions.
pub fn swizzle_surface(
    width: usize,
    height: usize,
    depth: usize,
    source: &[u8],
    block_dim: BlockDim, // TODO: Use None to indicate uncompressed?
    block_height_mip0: Option<BlockHeight>, // TODO: Make this optional in other functions as well?
    bytes_per_pixel: usize,
    mipmap_count: usize,
    array_count: usize,
) -> Result<Vec<u8>, SwizzleError> {
    swizzle_surface_inner::<false>(
        width,
        height,
        depth,
        source,
        block_dim,
        block_height_mip0,
        bytes_per_pixel,
        mipmap_count,
        array_count,
    )
}

// TODO: Find a way to simplify the parameters.
/// Deswizzles all the array layers and mipmaps in `source` using the block linear algorithm
/// to a new vector without any padding between array layers or mipmaps.
///
/// Set `block_height_mip0` to [None] to infer the block height from the specified dimensions.
pub fn deswizzle_surface(
    width: usize,
    height: usize,
    depth: usize,
    source: &[u8],
    block_dim: BlockDim,
    block_height_mip0: Option<BlockHeight>, // TODO: Make this optional in other functions as well?
    bytes_per_pixel: usize,
    mipmap_count: usize,
    array_count: usize,
) -> Result<Vec<u8>, SwizzleError> {
    swizzle_surface_inner::<true>(
        width,
        height,
        depth,
        source,
        block_dim,
        block_height_mip0,
        bytes_per_pixel,
        mipmap_count,
        array_count,
    )
}

fn swizzle_surface_inner<const DESWIZZLE: bool>(
    width: usize,
    height: usize,
    depth: usize,
    source: &[u8],
    block_dim: BlockDim,
    block_height_mip0: Option<BlockHeight>, // TODO: Make this optional in other functions as well?
    bytes_per_pixel: usize,
    mipmap_count: usize,
    array_count: usize,
) -> Result<Vec<u8>, SwizzleError> {
    // TODO: 3D support.
    // TODO: We can assume the total size is 33% larger than the base level.
    // This should eliminate any reallocations.
    let mut result = Vec::new();

    let block_width = block_dim.width.get();
    let block_height = block_dim.height.get();

    // The block height can be inferred if not specified.
    let block_height_mip0 = block_height_mip0
        .unwrap_or_else(|| crate::block_height_mip0(div_round_up(height, block_height)));

    let mut offset = 0;
    for _ in 0..array_count {
        for mip in 0..mipmap_count {
            let mip_width = max(div_round_up(width >> mip, block_width), 1);
            let mip_height = max(div_round_up(height >> mip, block_height), 1);

            // The block height will likely change for each mip level.
            let mip_block_height = mip_block_height(mip_height, block_height_mip0);

            // TODO: Use the inner function here for fewer heap allocations.
            let mipmap_data = if DESWIZZLE {
                deswizzle_block_linear(
                    mip_width,
                    mip_height,
                    1,
                    &source[offset..], // TODO: Potential panic?
                    mip_block_height,
                    bytes_per_pixel,
                )?
            } else {
                swizzle_block_linear(
                    mip_width,
                    mip_height,
                    1,
                    &source[offset..], // TODO: Potential panic?
                    mip_block_height,
                    bytes_per_pixel,
                )?
            };

            result.extend_from_slice(&mipmap_data);

            offset += if DESWIZZLE {
                swizzled_surface_size(mip_width, mip_height, 1, mip_block_height, bytes_per_pixel)
            } else {
                deswizzled_surface_size(mip_width, mip_height, 1, bytes_per_pixel)
            };
        }

        // Alignment for array layers.
        if array_count > 1 {
            if DESWIZZLE {
                // Align the swizzled source offset.
                offset = align_layer_size(
                    offset,
                    max(div_round_up(height, block_height), 1),
                    1,
                    block_height_mip0,
                    1,
                );
            } else {
                // Align the swizzled output data.
                let new_length = align_layer_size(
                    result.len(),
                    max(div_round_up(height, block_height), 1),
                    1,
                    block_height_mip0,
                    1,
                );
                result.resize(new_length, 0);
            }
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Use helper functions to shorten the test cases.
    fn swizzle_length(
        width: usize,
        height: usize,
        source_length: usize,
        is_compressed: bool,
        bpp: usize,
        layer_count: usize,
        mipmap_count: usize,
    ) -> usize {
        swizzle_surface(
            width,
            height,
            1,
            &vec![0u8; source_length],
            if is_compressed {
                BlockDim::block_4x4()
            } else {
                BlockDim::uncompressed()
            },
            None,
            bpp,
            layer_count,
            mipmap_count,
        )
        .unwrap()
        .len()
    }

    fn deswizzle_length(
        width: usize,
        height: usize,
        source_length: usize,
        is_compressed: bool,
        bpp: usize,
        layer_count: usize,
        mipmap_count: usize,
    ) -> usize {
        deswizzle_surface(
            width,
            height,
            1,
            &vec![0u8; source_length],
            if is_compressed {
                BlockDim::block_4x4()
            } else {
                BlockDim::uncompressed()
            },
            None,
            bpp,
            layer_count,
            mipmap_count,
        )
        .unwrap()
        .len()
    }

    // Expected swizzled sizes are taken from the nutexb footer.
    // Expected deswizzled sizes are the product of the mipmap size sum and the array count.
    #[test]
    fn swizzle_surface_arrays_no_mipmaps_length() {
        assert_eq!(6144, swizzle_length(16, 16, 6144, false, 4, 1, 6));
        assert_eq!(3072, swizzle_length(16, 16, 768, true, 8, 1, 6));
        assert_eq!(
            25165824,
            swizzle_length(2048, 2048, 25165824, true, 16, 1, 6)
        );
        assert_eq!(1572864, swizzle_length(256, 256, 1572864, false, 4, 1, 6));
        assert_eq!(98304, swizzle_length(64, 64, 98304, false, 4, 1, 6));
        assert_eq!(98304, swizzle_length(64, 64, 98304, false, 4, 1, 6));
        assert_eq!(393216, swizzle_length(64, 64, 393216, false, 16, 1, 6));
    }

    #[test]
    fn swizzle_surface_arrays_mipmaps_length() {
        assert_eq!(147456, swizzle_length(128, 128, 131232, true, 16, 8, 6));
        assert_eq!(15360, swizzle_length(16, 16, 2208, true, 16, 5, 6));
        assert_eq!(540672, swizzle_length(256, 256, 524448, true, 16, 9, 6));
        assert_eq!(1204224, swizzle_length(288, 288, 664512, true, 16, 9, 6));
        assert_eq!(2113536, swizzle_length(512, 512, 2097312, true, 16, 10, 6));
        assert_eq!(49152, swizzle_length(64, 64, 32928, true, 16, 7, 6));
    }

    #[test]
    fn swizzle_surface_nutexb_length() {
        // Sizes and parameters taken from Smash Ultimate nutexb files.
        // The deswizzled size is estimated as the product of the mip sizes sum and array count.
        // The swizzled size is taken from the footer.
        assert_eq!(12800, swizzle_length(100, 100, 6864, true, 8, 7, 1));
        assert_eq!(360960, swizzle_length(1028, 256, 351376, true, 16, 11, 1));
        assert_eq!(24064, swizzle_length(128, 32, 21852, false, 4, 8, 1));
        assert_eq!(
            2099712,
            swizzle_length(1536, 1024, 2097184, true, 16, 11, 1)
        );
        assert_eq!(35328, swizzle_length(180, 180, 21992, true, 8, 8, 1));
        assert_eq!(
            4546048,
            swizzle_length(2048, 1344, 3670320, true, 16, 12, 1)
        );
        assert_eq!(17920, swizzle_length(256, 32, 11024, true, 16, 9, 1));
        assert_eq!(58368, swizzle_length(320, 128, 54672, true, 16, 9, 1));
        assert_eq!(125440, swizzle_length(340, 340, 77840, true, 8, 9, 1));
        assert_eq!(147968, swizzle_length(400, 400, 106864, true, 8, 9, 1));
        assert_eq!(2048, swizzle_length(4, 24, 384, false, 4, 1, 1));
        assert_eq!(351744, swizzle_length(512, 384, 262192, true, 16, 10, 1));
        assert_eq!(440832, swizzle_length(640, 640, 273120, true, 8, 10, 1));
        assert_eq!(26624, swizzle_length(64, 512, 21896, true, 8, 10, 1));
        assert_eq!(280064, swizzle_length(800, 400, 213576, true, 8, 10, 1));
        assert_eq!(
            16777216,
            swizzle_length(8192, 2048, 16777216, true, 16, 1, 1)
        );
    }

    #[test]
    fn deswizzle_surface_nutexb_length() {
        // Sizes and parameters taken from Smash Ultimate nutexb files.
        // The deswizzled size is estimated as the product of the mip sizes sum and array count.
        // The swizzled size is taken from the footer.
        assert_eq!(6864, deswizzle_length(100, 100, 12800, true, 8, 7, 1));
        assert_eq!(351376, deswizzle_length(1028, 256, 360960, true, 16, 11, 1));
        assert_eq!(21852, deswizzle_length(128, 32, 24064, false, 4, 8, 1));
        assert_eq!(
            2097184,
            deswizzle_length(1536, 1024, 2099712, true, 16, 11, 1)
        );
        assert_eq!(21992, deswizzle_length(180, 180, 35328, true, 8, 8, 1));
        assert_eq!(
            3670320,
            deswizzle_length(2048, 1344, 4546048, true, 16, 12, 1)
        );
        assert_eq!(11024, deswizzle_length(256, 32, 17920, true, 16, 9, 1));
        assert_eq!(54672, deswizzle_length(320, 128, 58368, true, 16, 9, 1));
        assert_eq!(77840, deswizzle_length(340, 340, 125440, true, 8, 9, 1));
        assert_eq!(106864, deswizzle_length(400, 400, 147968, true, 8, 9, 1));
        assert_eq!(384, deswizzle_length(4, 24, 2048, false, 4, 1, 1));
        assert_eq!(262192, deswizzle_length(512, 384, 351744, true, 16, 10, 1));
        assert_eq!(273120, deswizzle_length(640, 640, 440832, true, 8, 10, 1));
        assert_eq!(21896, deswizzle_length(64, 512, 26624, true, 8, 10, 1));
        assert_eq!(213576, deswizzle_length(800, 400, 280064, true, 8, 10, 1));
        assert_eq!(
            16777216,
            deswizzle_length(8192, 2048, 16777216, true, 16, 1, 1)
        );
    }

    #[test]
    fn deswizzle_surface_arrays_no_mipmaps_length() {
        assert_eq!(6144, deswizzle_length(16, 16, 6144, false, 4, 1, 6));
        assert_eq!(768, deswizzle_length(16, 16, 3072, true, 8, 1, 6));
        assert_eq!(
            25165824,
            deswizzle_length(2048, 2048, 25165824, true, 16, 1, 6)
        );
        assert_eq!(1572864, deswizzle_length(256, 256, 1572864, false, 4, 1, 6));
        assert_eq!(98304, deswizzle_length(64, 64, 98304, false, 4, 1, 6));
        assert_eq!(98304, deswizzle_length(64, 64, 98304, false, 4, 1, 6));
        assert_eq!(393216, deswizzle_length(64, 64, 393216, false, 16, 1, 6));
    }

    #[test]
    fn deswizzle_surface_arrays_mipmaps_length() {
        assert_eq!(131232, deswizzle_length(128, 128, 147456, true, 16, 8, 6));
        assert_eq!(2208, deswizzle_length(16, 16, 15360, true, 16, 5, 6));
        assert_eq!(524448, deswizzle_length(256, 256, 540672, true, 16, 9, 6));
        assert_eq!(664512, deswizzle_length(288, 288, 1204224, true, 16, 9, 6));
        assert_eq!(
            2097312,
            deswizzle_length(512, 512, 2113536, true, 16, 10, 6)
        );
        assert_eq!(32928, deswizzle_length(64, 64, 49152, true, 16, 7, 6));
    }
}
