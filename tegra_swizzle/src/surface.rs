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
    for array in 0..array_count {
        for mip in 0..mipmap_count {
            let mip_width = max(div_round_up(width >> mip, block_width), 1);
            let mip_height = max(div_round_up(height >> mip, block_height), 1);

            // The block height will likely change for each mip level.
            let mip_block_height = mip_block_height(mip_height, block_height_mip0);

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

    // Expected swizzled sizes are taken from the nutexb footer.
    // Expected deswizzled sizes are the product of the mipmap size sum and the array count.
    #[test]
    fn swizzle_data_arrays_no_mipmaps_length() {
        assert_eq!(
            6144,
            swizzle_surface(
                16,
                16,
                1,
                &[0u8; 6144],
                BlockDim::uncompressed(),
                None,
                4,
                1,
                6
            )
            .unwrap()
            .len()
        );
        assert_eq!(
            3072,
            swizzle_surface(16, 16, 1, &[0u8; 768], BlockDim::block_4x4(), None, 8, 1, 6)
                .unwrap()
                .len()
        );
        assert_eq!(
            25165824,
            swizzle_surface(
                2048,
                2048,
                1,
                &[0u8; 25165824],
                BlockDim::block_4x4(),
                None,
                16,
                1,
                6
            )
            .unwrap()
            .len()
        );
        assert_eq!(
            1572864,
            swizzle_surface(
                256,
                256,
                1,
                &[0u8; 1572864],
                BlockDim::uncompressed(),
                None,
                4,
                1,
                6
            )
            .unwrap()
            .len()
        );
        assert_eq!(
            98304,
            swizzle_surface(
                64,
                64,
                1,
                &[0u8; 98304],
                BlockDim::uncompressed(),
                None,
                4,
                1,
                6
            )
            .unwrap()
            .len()
        );
        assert_eq!(
            98304,
            swizzle_surface(
                64,
                64,
                1,
                &[0u8; 98304],
                BlockDim::uncompressed(),
                None,
                4,
                1,
                6
            )
            .unwrap()
            .len()
        );
        assert_eq!(
            393216,
            swizzle_surface(
                64,
                64,
                1,
                &[0u8; 393216],
                BlockDim::uncompressed(),
                None,
                16,
                1,
                6
            )
            .unwrap()
            .len()
        );
    }

    #[test]
    fn swizzle_data_arrays_mipmaps_length() {
        assert_eq!(
            147456,
            swizzle_surface(
                128,
                128,
                1,
                &[0u8; 131232],
                BlockDim::block_4x4(),
                None,
                16,
                8,
                6
            )
            .unwrap()
            .len()
        );
        assert_eq!(
            15360,
            swizzle_surface(
                16,
                16,
                1,
                &[0u8; 2208],
                BlockDim::block_4x4(),
                None,
                16,
                5,
                6
            )
            .unwrap()
            .len()
        );
        assert_eq!(
            540672,
            swizzle_surface(
                256,
                256,
                1,
                &[0u8; 524448],
                BlockDim::block_4x4(),
                None,
                16,
                9,
                6
            )
            .unwrap()
            .len()
        );
        assert_eq!(
            1204224,
            swizzle_surface(
                288,
                288,
                1,
                &[0u8; 664512],
                BlockDim::block_4x4(),
                None,
                16,
                9,
                6
            )
            .unwrap()
            .len()
        );
        assert_eq!(
            2113536,
            swizzle_surface(
                512,
                512,
                1,
                &[0u8; 2097312],
                BlockDim::block_4x4(),
                None,
                16,
                10,
                6
            )
            .unwrap()
            .len()
        );
        assert_eq!(
            49152,
            swizzle_surface(
                64,
                64,
                1,
                &[0u8; 32928],
                BlockDim::block_4x4(),
                None,
                16,
                7,
                6
            )
            .unwrap()
            .len()
        );
    }

    #[test]
    fn swizzle_data_nutexb_length() {
        // Sizes and parameters taken from Smash Ultimate nutexb files.
        // The deswizzled size is estimated as the product of the mip sizes sum and array count.
        // The swizzled size is taken from the footer.
        assert_eq!(
            1024,
            swizzle_surface(
                16,
                16,
                1,
                &[0u8; 1024],
                BlockDim::uncompressed(),
                None,
                4,
                1,
                1
            )
            .unwrap()
            .len()
        );
        assert_eq!(
            147968,
            swizzle_surface(
                400,
                360,
                1,
                &[0u8; 96304],
                BlockDim::block_4x4(),
                None,
                8,
                9,
                1
            )
            .unwrap()
            .len()
        );
        assert_eq!(
            176640,
            swizzle_surface(
                256,
                384,
                1,
                &[0u8; 131104],
                BlockDim::block_4x4(),
                None,
                16,
                9,
                1
            )
            .unwrap()
            .len()
        );
        assert_eq!(
            20480,
            swizzle_surface(
                96,
                256,
                1,
                &[0u8; 16424],
                BlockDim::block_4x4(),
                None,
                8,
                9,
                1
            )
            .unwrap()
            .len()
        );
        assert_eq!(
            21504,
            swizzle_surface(
                100,
                100,
                1,
                &[0u8; 13728],
                BlockDim::block_4x4(),
                None,
                16,
                7,
                1
            )
            .unwrap()
            .len()
        );
        assert_eq!(
            22371840,
            swizzle_surface(
                4096,
                4096,
                1,
                &[0u8; 22369648],
                BlockDim::block_4x4(),
                None,
                16,
                13,
                1
            )
            .unwrap()
            .len()
        );
        assert_eq!(
            256000,
            swizzle_surface(
                360,
                300,
                1,
                &[0u8; 144960],
                BlockDim::block_4x4(),
                None,
                16,
                9,
                1
            )
            .unwrap()
            .len()
        );
        assert_eq!(
            2624512,
            swizzle_surface(
                1920,
                848,
                1,
                &[0u8; 2171984],
                BlockDim::block_4x4(),
                None,
                16,
                11,
                1
            )
            .unwrap()
            .len()
        );
        assert_eq!(
            2949120,
            swizzle_surface(
                2880,
                1632,
                1,
                &[0u8; 2350080],
                BlockDim::block_4x4(),
                None,
                8,
                1,
                1
            )
            .unwrap()
            .len()
        );
        assert_eq!(
            31232,
            swizzle_surface(
                148,
                148,
                1,
                &[0u8; 14936],
                BlockDim::block_4x4(),
                None,
                8,
                8,
                1
            )
            .unwrap()
            .len()
        );
        assert_eq!(
            34816,
            swizzle_surface(
                4,
                1024,
                1,
                &[0u8; 4104],
                BlockDim::block_4x4(),
                None,
                8,
                11,
                1
            )
            .unwrap()
            .len()
        );
        assert_eq!(
            5594624,
            swizzle_surface(
                2048,
                2048,
                1,
                &[0u8; 5592432],
                BlockDim::block_4x4(),
                None,
                16,
                12,
                1
            )
            .unwrap()
            .len()
        );
        assert_eq!(
            5632,
            swizzle_surface(
                64,
                16,
                1,
                &[0u8; 1424],
                BlockDim::block_4x4(),
                None,
                16,
                7,
                1
            )
            .unwrap()
            .len()
        );
        assert_eq!(
            5632,
            swizzle_surface(64, 8, 1, &[0u8; 784], BlockDim::block_4x4(), None, 16, 7, 1)
                .unwrap()
                .len()
        );
        assert_eq!(
            700928,
            swizzle_surface(
                512,
                768,
                1,
                &[0u8; 524320],
                BlockDim::block_4x4(),
                None,
                16,
                10,
                1
            )
            .unwrap()
            .len()
        );
        assert_eq!(
            701952,
            swizzle_surface(
                1024,
                512,
                1,
                &[0u8; 699088],
                BlockDim::block_4x4(),
                None,
                16,
                11,
                1
            )
            .unwrap()
            .len()
        );
        assert_eq!(
            89088,
            swizzle_surface(
                128,
                128,
                1,
                &[0u8; 87380],
                BlockDim::uncompressed(),
                None,
                4,
                8,
                1
            )
            .unwrap()
            .len()
        );
        assert_eq!(
            90112,
            swizzle_surface(
                512,
                256,
                1,
                &[0u8; 87400],
                BlockDim::block_4x4(),
                None,
                8,
                10,
                1
            )
            .unwrap()
            .len()
        );
    }

    #[test]
    fn deswizzle_data_nutexb_length() {
        // Sizes and parameters taken from Smash Ultimate nutexb files.
        // The deswizzled size is estimated as the product of the mip sizes sum and array count.
        // The swizzled size is taken from the footer.
        assert_eq!(
            1024,
            deswizzle_surface(
                16,
                16,
                1,
                &[0u8; 1024],
                BlockDim::uncompressed(),
                None,
                4,
                1,
                1
            )
            .unwrap()
            .len()
        );
        assert_eq!(
            131104,
            deswizzle_surface(
                256,
                384,
                1,
                &[0u8; 176640],
                BlockDim::block_4x4(),
                None,
                16,
                9,
                1
            )
            .unwrap()
            .len()
        );
        assert_eq!(
            13728,
            deswizzle_surface(
                100,
                100,
                1,
                &[0u8; 21504],
                BlockDim::block_4x4(),
                None,
                16,
                7,
                1
            )
            .unwrap()
            .len()
        );
        assert_eq!(
            1424,
            deswizzle_surface(
                64,
                16,
                1,
                &[0u8; 5632],
                BlockDim::block_4x4(),
                None,
                16,
                7,
                1
            )
            .unwrap()
            .len()
        );
        assert_eq!(
            144960,
            deswizzle_surface(
                360,
                300,
                1,
                &[0u8; 256000],
                BlockDim::block_4x4(),
                None,
                16,
                9,
                1
            )
            .unwrap()
            .len()
        );
        assert_eq!(
            14936,
            deswizzle_surface(
                148,
                148,
                1,
                &[0u8; 31232],
                BlockDim::block_4x4(),
                None,
                8,
                8,
                1
            )
            .unwrap()
            .len()
        );
        assert_eq!(
            16424,
            deswizzle_surface(
                96,
                256,
                1,
                &[0u8; 20480],
                BlockDim::block_4x4(),
                None,
                8,
                9,
                1
            )
            .unwrap()
            .len()
        );
        assert_eq!(
            2171984,
            deswizzle_surface(
                1920,
                848,
                1,
                &[0u8; 2624512],
                BlockDim::block_4x4(),
                None,
                16,
                11,
                1
            )
            .unwrap()
            .len()
        );
        assert_eq!(
            22369648,
            deswizzle_surface(
                4096,
                4096,
                1,
                &[0u8; 22371840],
                BlockDim::block_4x4(),
                None,
                16,
                13,
                1
            )
            .unwrap()
            .len()
        );
        assert_eq!(
            2350080,
            deswizzle_surface(
                2880,
                1632,
                1,
                &[0u8; 2949120],
                BlockDim::block_4x4(),
                None,
                8,
                1,
                1
            )
            .unwrap()
            .len()
        );
        assert_eq!(
            4104,
            deswizzle_surface(
                4,
                1024,
                1,
                &[0u8; 34816],
                BlockDim::block_4x4(),
                None,
                8,
                11,
                1
            )
            .unwrap()
            .len()
        );
        assert_eq!(
            524320,
            deswizzle_surface(
                512,
                768,
                1,
                &[0u8; 700928],
                BlockDim::block_4x4(),
                None,
                16,
                10,
                1
            )
            .unwrap()
            .len()
        );
        assert_eq!(
            5592432,
            deswizzle_surface(
                2048,
                2048,
                1,
                &[0u8; 5594624],
                BlockDim::block_4x4(),
                None,
                16,
                12,
                1
            )
            .unwrap()
            .len()
        );
        assert_eq!(
            699088,
            deswizzle_surface(
                1024,
                512,
                1,
                &[0u8; 701952],
                BlockDim::block_4x4(),
                None,
                16,
                11,
                1
            )
            .unwrap()
            .len()
        );
        assert_eq!(
            784,
            deswizzle_surface(
                64,
                8,
                1,
                &[0u8; 5632],
                BlockDim::block_4x4(),
                None,
                16,
                7,
                1
            )
            .unwrap()
            .len()
        );
        assert_eq!(
            87380,
            deswizzle_surface(
                128,
                128,
                1,
                &[0u8; 89088],
                BlockDim::uncompressed(),
                None,
                4,
                8,
                1
            )
            .unwrap()
            .len()
        );
        assert_eq!(
            87400,
            deswizzle_surface(
                512,
                256,
                1,
                &[0u8; 90112],
                BlockDim::block_4x4(),
                None,
                8,
                10,
                1
            )
            .unwrap()
            .len()
        );
        assert_eq!(
            96304,
            deswizzle_surface(
                400,
                360,
                1,
                &[0u8; 147968],
                BlockDim::block_4x4(),
                None,
                8,
                9,
                1
            )
            .unwrap()
            .len()
        );
    }

    #[test]
    fn deswizzle_data_arrays_no_mipmaps_length() {
        assert_eq!(
            6144,
            deswizzle_surface(
                16,
                16,
                1,
                &[0u8; 6144],
                BlockDim::uncompressed(),
                None,
                4,
                1,
                6
            )
            .unwrap()
            .len()
        );
        assert_eq!(
            768,
            deswizzle_surface(
                16,
                16,
                1,
                &[0u8; 3072],
                BlockDim::block_4x4(),
                None,
                8,
                1,
                6
            )
            .unwrap()
            .len()
        );
        assert_eq!(
            25165824,
            deswizzle_surface(
                2048,
                2048,
                1,
                &[0u8; 25165824],
                BlockDim::block_4x4(),
                None,
                16,
                1,
                6
            )
            .unwrap()
            .len()
        );
        assert_eq!(
            1572864,
            deswizzle_surface(
                256,
                256,
                1,
                &[0u8; 1572864],
                BlockDim::uncompressed(),
                None,
                4,
                1,
                6
            )
            .unwrap()
            .len()
        );
        assert_eq!(
            98304,
            deswizzle_surface(
                64,
                64,
                1,
                &[0u8; 98304],
                BlockDim::uncompressed(),
                None,
                4,
                1,
                6
            )
            .unwrap()
            .len()
        );
        assert_eq!(
            98304,
            deswizzle_surface(
                64,
                64,
                1,
                &[0u8; 98304],
                BlockDim::uncompressed(),
                None,
                4,
                1,
                6
            )
            .unwrap()
            .len()
        );
        assert_eq!(
            393216,
            deswizzle_surface(
                64,
                64,
                1,
                &[0u8; 393216],
                BlockDim::uncompressed(),
                None,
                16,
                1,
                6
            )
            .unwrap()
            .len()
        );
    }

    #[test]
    fn deswizzle_data_arrays_mipmaps_length() {
        assert_eq!(
            131232,
            deswizzle_surface(
                128,
                128,
                1,
                &[0u8; 147456],
                BlockDim::block_4x4(),
                None,
                16,
                8,
                6
            )
            .unwrap()
            .len()
        );
        assert_eq!(
            2208,
            deswizzle_surface(
                16,
                16,
                1,
                &[0u8; 15360],
                BlockDim::block_4x4(),
                None,
                16,
                5,
                6
            )
            .unwrap()
            .len()
        );
        assert_eq!(
            524448,
            deswizzle_surface(
                256,
                256,
                1,
                &[0u8; 540672],
                BlockDim::block_4x4(),
                None,
                16,
                9,
                6
            )
            .unwrap()
            .len()
        );
        assert_eq!(
            664512,
            deswizzle_surface(
                288,
                288,
                1,
                &[0u8; 1204224],
                BlockDim::block_4x4(),
                None,
                16,
                9,
                6
            )
            .unwrap()
            .len()
        );
        assert_eq!(
            2097312,
            deswizzle_surface(
                512,
                512,
                1,
                &[0u8; 2113536],
                BlockDim::block_4x4(),
                None,
                16,
                10,
                6
            )
            .unwrap()
            .len()
        );
        assert_eq!(
            32928,
            deswizzle_surface(
                64,
                64,
                1,
                &[0u8; 49152],
                BlockDim::block_4x4(),
                None,
                16,
                7,
                6
            )
            .unwrap()
            .len()
        );
    }
}
