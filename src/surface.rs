//! Functions for working with surfaces stored in a combined buffer for all array layers and mipmaps.
//!
//! It's common for texture surfaces to be represented
//! as a single allocated region of memory that contains all array layers and mipmaps.
//! This also applies to the tiled surfaces used for most textures on the Tegra X1.
//!
//! Use [deswizzle_surface] for untiling surfaces into a single `Vec<u8>`.
//! This output can be used as is for creating DDS files.
//! Modern graphics APIs like Vulkan also support this dense layout for initializing all
//! array layers and mipmaps for a texture in a single API call.
//!
//! Use [swizzle_surface] for tiling a surface from a combined buffer like the result of [deswizzle_surface] or a DDS file.
//! The result of [swizzle_surface] is the layout expected for many texture file formats for console games targeting the Tegra X1.
//!
//! # Examples
//! Array layers and mipmaps are ordered by layer and then mipmap.
//! A surface with `L` layers and `M` mipmaps would have the following layout.
/*!
```no_compile
Layer 0 Mip 0
Layer 0 Mip 1
...
Layer 0 Mip M-1
Layer 1 Mip 0
Layer 1 Mip 1
...
Layer L-1 Mip M-1
```
*/
//! The convention is for the untiled or linear layout to be tightly packed.
//! Tiled surfaces add additional padding and alignment between layers and mipmaps.
use alloc::{vec, vec::Vec};
use core::{cmp::max, num::NonZeroU32};

use crate::{
    arrays::align_layer_size,
    blockdepth::mip_block_depth,
    div_round_up, mip_block_height,
    swizzle::{deswizzled_mip_size, swizzle_inner, swizzled_mip_size},
    BlockHeight, SwizzleError,
};

/// The dimensions of a compressed block. Compressed block sizes are usually 4x4 pixels.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockDim {
    /// The width of the block in pixels.
    pub width: NonZeroU32,
    /// The height of the block in pixels.
    pub height: NonZeroU32,
    /// The depth of the block in pixels.
    pub depth: NonZeroU32,
}

impl BlockDim {
    /// A 1x1x1 block for formats that do not use block compression like R8G8B8A8.
    pub fn uncompressed() -> Self {
        BlockDim {
            width: NonZeroU32::new(1).unwrap(),
            height: NonZeroU32::new(1).unwrap(),
            depth: NonZeroU32::new(1).unwrap(),
        }
    }

    /// A 4x4x1 compressed block. This includes any of the BCN formats like BC1, BC3, or BC7.
    /// This also includes DXT1, DXT3, and DXT5.
    pub fn block_4x4() -> Self {
        BlockDim {
            width: NonZeroU32::new(4).unwrap(),
            height: NonZeroU32::new(4).unwrap(),
            depth: NonZeroU32::new(1).unwrap(),
        }
    }
}

/// Tiles all the array layers and mipmaps in `source` using the block linear algorithm
/// to a combined vector with appropriate mipmap and layer alignment.
///
/// The `width`, `height`, and `depth` are in terms of blocks with the pixels per block defined by `block_dim`.
/// Use a `block_height_mip0` of [None] to infer the block height from the specified dimensions.
///
/// Returns [SwizzleError::NotEnoughData] if `source` does not have
/// at least as many bytes as the result of [deswizzled_surface_size].
///
/// # Examples
///
/// Compressed formats should still use pixel dimensions and set the appropriate block dimensions.
///
/// ```rust no_run
/// use tegra_swizzle::surface::{BlockDim, swizzle_surface};
/// # let deswizzled_surface = vec![0u8; 10];
///
/// // 16x16 BC7 cube map with 5 mipmaps.
/// let surface = swizzle_surface(
///     16,
///     16,
///     1,
///     &deswizzled_surface,
///     BlockDim::block_4x4(),
///     None,
///     16,
///     5,
///     6,
/// );
/// ```
///
/// Uncompressed formats use a 1x1x1 pixel block.
///
/// ```rust no_run
/// // 128x128 R8G8B8A8 2D texture with no mipmaps.
/// use tegra_swizzle::surface::{BlockDim, swizzle_surface};
/// # let deswizzled_surface = vec![0u8; 10];
/// let surface = swizzle_surface(
///     128,
///     128,
///     1,
///     &deswizzled_surface,
///     BlockDim::uncompressed(),
///     None,
///     4,
///     1,
///     1,
/// );
///
/// // 16x16x16 R8G8B8A8 3D texture with no mipmaps.
/// let surface = swizzle_surface(
///     16,
///     16,
///     16,
///     &deswizzled_surface,
///     BlockDim::uncompressed(),
///     None,
///     4,
///     1,
///     1,
/// );
/// ```
pub fn swizzle_surface(
    width: u32,
    height: u32,
    depth: u32,
    source: &[u8],
    block_dim: BlockDim, // TODO: Use None to indicate uncompressed?
    block_height_mip0: Option<BlockHeight>, // TODO: Make this optional in other functions as well?
    bytes_per_pixel: u32,
    mipmap_count: u32,
    layer_count: u32,
) -> Result<Vec<u8>, SwizzleError> {
    // Check for empty surfaces first to more reliably handle overflow.
    if width == 0
        || height == 0
        || depth == 0
        || bytes_per_pixel == 0
        || mipmap_count == 0
        || layer_count == 0
    {
        return Ok(Vec::new());
    }

    validate_surface(width, height, depth, bytes_per_pixel, mipmap_count)?;

    let mut result = surface_destination::<false>(
        width,
        height,
        depth,
        block_dim,
        block_height_mip0,
        bytes_per_pixel,
        mipmap_count,
        layer_count,
        source,
    )?;

    swizzle_surface_inner::<false>(
        width,
        height,
        depth,
        source,
        &mut result,
        block_dim,
        block_height_mip0,
        bytes_per_pixel,
        mipmap_count,
        layer_count,
    )?;

    Ok(result)
}

// TODO: Find a way to simplify the parameters.
/// Untiles all the array layers and mipmaps in `source` using the block linear algorithm
/// to a new vector without any padding between layers or mipmaps.
///
/// The `width`, `height`, and `depth` are in terms of blocks with the pixels per block defined by `block_dim`.
/// Use a `block_height_mip0` of [None] to infer the block height from the specified dimensions.
///
/// Returns [SwizzleError::NotEnoughData] if `source` does not have
/// at least as many bytes as the result of [swizzled_surface_size].
///
/// # Examples
///
/// Compressed formats should still use pixel dimensions and set the appropriate block dimensions.
///
/// ```rust no_run
/// use tegra_swizzle::surface::{BlockDim, deswizzle_surface};
/// # let swizzled_surface = vec![0u8; 10];
///
/// // 16x16 BC7 cube map with 5 mipmaps.
/// let surface = deswizzle_surface(
///     16,
///     16,
///     1,
///     &swizzled_surface,
///     BlockDim::block_4x4(),
///     None,
///     16,
///     5,
///     6,
/// );
/// ```
///
/// Uncompressed formats use a 1x1x1 pixel block.
///
/// ```rust no_run
/// use tegra_swizzle::surface::{BlockDim, deswizzle_surface};
/// # let swizzled_surface = vec![0u8; 10];
/// // 128x128 R8G8B8A8 2D texture with no mipmaps.
/// let surface = deswizzle_surface(
///     128,
///     128,
///     1,
///     &swizzled_surface,
///     BlockDim::uncompressed(),
///     None,
///     4,
///     1,
///     1,
/// );
///
/// // 16x16x16 R8G8B8A8 3D texture with no mipmaps.
/// let surface = deswizzle_surface(
///     16,
///     16,
///     16,
///     &swizzled_surface,
///     BlockDim::uncompressed(),
///     None,
///     4,
///     1,
///     1,
/// );
/// ```
pub fn deswizzle_surface(
    width: u32,
    height: u32,
    depth: u32,
    source: &[u8],
    block_dim: BlockDim,
    block_height_mip0: Option<BlockHeight>, // TODO: Make this optional in other functions as well?
    bytes_per_pixel: u32,
    mipmap_count: u32,
    layer_count: u32,
) -> Result<Vec<u8>, SwizzleError> {
    // Check for empty surfaces first to more reliably handle overflow.
    if width == 0
        || height == 0
        || depth == 0
        || bytes_per_pixel == 0
        || mipmap_count == 0
        || layer_count == 0
    {
        return Ok(Vec::new());
    }

    validate_surface(width, height, depth, bytes_per_pixel, mipmap_count)?;

    let mut result = surface_destination::<true>(
        width,
        height,
        depth,
        block_dim,
        block_height_mip0,
        bytes_per_pixel,
        mipmap_count,
        layer_count,
        source,
    )?;

    swizzle_surface_inner::<true>(
        width,
        height,
        depth,
        source,
        &mut result,
        block_dim,
        block_height_mip0,
        bytes_per_pixel,
        mipmap_count,
        layer_count,
    )?;

    Ok(result)
}

pub(crate) fn swizzle_surface_inner<const DESWIZZLE: bool>(
    width: u32,
    height: u32,
    depth: u32,
    source: &[u8],
    result: &mut [u8],
    block_dim: BlockDim,
    block_height_mip0: Option<BlockHeight>, // TODO: Make this optional in other functions as well?
    bytes_per_pixel: u32,
    mipmap_count: u32,
    layer_count: u32,
) -> Result<(), SwizzleError> {
    let block_width = block_dim.width.get();
    let block_height = block_dim.height.get();
    let block_depth = block_dim.depth.get();

    // The block height can be inferred if not specified.
    // TODO: Enforce a block height of 1 for depth textures elsewhere?
    let block_height_mip0 = if depth == 1 {
        block_height_mip0
            .unwrap_or_else(|| crate::block_height_mip0(div_round_up(height, block_height)))
    } else {
        BlockHeight::One
    };

    // TODO: Don't assume block_depth is 1?
    let block_depth_mip0 = crate::blockdepth::block_depth(depth);

    let mut src_offset = 0;
    let mut dst_offset = 0;
    for _ in 0..layer_count {
        for mip in 0..mipmap_count {
            let mip_width = max(div_round_up(width >> mip, block_width), 1);
            let mip_height = max(div_round_up(height >> mip, block_height), 1);
            let mip_depth = max(div_round_up(depth >> mip, block_depth), 1);

            let mip_block_height = mip_block_height(mip_height, block_height_mip0);
            let mip_block_depth = mip_block_depth(mip_depth, block_depth_mip0);

            swizzle_mipmap::<DESWIZZLE>(
                mip_width,
                mip_height,
                mip_depth,
                mip_block_height,
                mip_block_depth,
                bytes_per_pixel,
                source,
                &mut src_offset,
                result,
                &mut dst_offset,
            )?;
        }

        // Align offsets between array layers.
        if layer_count > 1 {
            if DESWIZZLE {
                src_offset = align_layer_size(src_offset, height, depth, block_height_mip0, 1);
            } else {
                dst_offset = align_layer_size(dst_offset, height, depth, block_height_mip0, 1);
            }
        }
    }

    Ok(())
}

fn surface_destination<const DESWIZZLE: bool>(
    width: u32,
    height: u32,
    depth: u32,
    block_dim: BlockDim,
    block_height_mip0: Option<BlockHeight>,
    bytes_per_pixel: u32,
    mipmap_count: u32,
    layer_count: u32,
    source: &[u8],
) -> Result<Vec<u8>, SwizzleError> {
    let swizzled_size = swizzled_surface_size(
        width,
        height,
        depth,
        block_dim,
        block_height_mip0,
        bytes_per_pixel,
        mipmap_count,
        layer_count,
    );
    let deswizzled_size = deswizzled_surface_size(
        width,
        height,
        depth,
        block_dim,
        bytes_per_pixel,
        mipmap_count,
        layer_count,
    );
    let (surface_size, expected_size) = if DESWIZZLE {
        (deswizzled_size, swizzled_size)
    } else {
        (swizzled_size, deswizzled_size)
    };

    // Validate the source length before attempting to allocate.
    // This reduces potential out of memory panics.
    if source.len() < expected_size {
        return Err(SwizzleError::NotEnoughData {
            actual_size: source.len(),
            expected_size,
        });
    }

    // Assume the calculated size is accurate, so don't reallocate later.
    Ok(vec![0u8; surface_size])
}

fn validate_surface(
    width: u32,
    height: u32,
    depth: u32,
    bytes_per_pixel: u32,
    mipmap_count: u32,
) -> Result<(), SwizzleError> {
    // Check dimensions to prevent overflow.
    if width
        .checked_mul(height)
        .and_then(|u| u.checked_mul(depth))
        .and_then(|u| u.checked_mul(bytes_per_pixel))
        .is_none()
        || width.checked_mul(bytes_per_pixel).is_none()
        || depth.checked_add(depth / 2).is_none()
        || mipmap_count > u32::BITS
    {
        Err(SwizzleError::InvalidSurface {
            width,
            height,
            depth,
            bytes_per_pixel,
            mipmap_count,
        })
    } else {
        Ok(())
    }
}

// TODO: Add examples.
/// Calculates the size in bytes for the tiled data for the given surface.
/// Compare with [deswizzled_surface_size].
///
/// Dimensions should be in pixels.
///
/// Use a `block_height_mip0` of [None] to infer the block height from the specified dimensions.
pub fn swizzled_surface_size(
    width: u32,
    height: u32,
    depth: u32,
    block_dim: BlockDim, // TODO: Use None to indicate uncompressed?
    block_height_mip0: Option<BlockHeight>, // TODO: Make this optional in other functions as well?
    bytes_per_pixel: u32,
    mipmap_count: u32,
    layer_count: u32,
) -> usize {
    let block_width = block_dim.width.get();
    let block_height = block_dim.height.get();
    let block_depth = block_dim.depth.get();

    // The block height can be inferred if not specified.
    // TODO: Enforce a block height of 1 for depth textures elsewhere?
    let block_height_mip0 = if depth == 1 {
        block_height_mip0
            .unwrap_or_else(|| crate::block_height_mip0(div_round_up(height, block_height)))
    } else {
        BlockHeight::One
    };

    let mut mip_size = 0;
    for mip in 0..mipmap_count {
        let mip_width = max(div_round_up(width >> mip, block_width), 1);
        let mip_height = max(div_round_up(height >> mip, block_height), 1);
        let mip_depth = max(div_round_up(depth >> mip, block_depth), 1);
        let mip_block_height = mip_block_height(mip_height, block_height_mip0);

        mip_size += swizzled_mip_size(
            mip_width,
            mip_height,
            mip_depth,
            mip_block_height,
            bytes_per_pixel,
        )
    }

    if layer_count > 1 {
        // We only need alignment between layers.
        let layer_size = align_layer_size(mip_size, height, depth, block_height_mip0, 1);
        layer_size * layer_count as usize
    } else {
        mip_size
    }
}

// TODO: Add examples.
/// Calculates the size in bytes for the untiled or linear data for the given surface.
/// Compare with [swizzled_surface_size].
///
/// Dimensions should be in pixels.
pub fn deswizzled_surface_size(
    width: u32,
    height: u32,
    depth: u32,
    block_dim: BlockDim,
    bytes_per_pixel: u32,
    mipmap_count: u32,
    layer_count: u32,
) -> usize {
    // TODO: Avoid duplicating this code.
    let block_width = block_dim.width.get();
    let block_height = block_dim.height.get();
    let block_depth = block_dim.depth.get();

    let mut layer_size = 0;
    for mip in 0..mipmap_count {
        let mip_width = max(div_round_up(width >> mip, block_width), 1);
        let mip_height = max(div_round_up(height >> mip, block_height), 1);
        let mip_depth = max(div_round_up(depth >> mip, block_depth), 1);
        layer_size += deswizzled_mip_size(mip_width, mip_height, mip_depth, bytes_per_pixel)
    }

    layer_size * layer_count as usize
}

fn swizzle_mipmap<const DESWIZZLE: bool>(
    with: u32,
    height: u32,
    depth: u32,
    block_height: BlockHeight,
    block_depth: u32,
    bytes_per_pixel: u32,
    source: &[u8],
    src_offset: &mut usize,
    dst: &mut [u8],
    dst_offset: &mut usize,
) -> Result<(), SwizzleError> {
    let swizzled_size = swizzled_mip_size(with, height, depth, block_height, bytes_per_pixel);
    let deswizzled_size = deswizzled_mip_size(with, height, depth, bytes_per_pixel);

    // Make sure the source has enough space.
    if DESWIZZLE && source.len() < *src_offset + swizzled_size {
        return Err(SwizzleError::NotEnoughData {
            expected_size: swizzled_size,
            actual_size: source.len(),
        });
    }

    if !DESWIZZLE && source.len() < *src_offset + deswizzled_size {
        return Err(SwizzleError::NotEnoughData {
            expected_size: deswizzled_size,
            actual_size: source.len(),
        });
    }

    // Tile or untile the data and move to the next section.
    swizzle_inner::<DESWIZZLE>(
        with,
        height,
        depth,
        &source[*src_offset..],
        &mut dst[*dst_offset..],
        block_height,
        block_depth,
        bytes_per_pixel,
    );

    if DESWIZZLE {
        *src_offset += swizzled_size;
        *dst_offset += deswizzled_size;
    } else {
        *src_offset += deswizzled_size;
        *dst_offset += swizzled_size;
    };

    Ok(())
}

#[cfg(test)]
mod tests {
    use core::u32;

    use super::*;

    // Use helper functions to shorten the test cases.
    fn swizzle_length(
        width: u32,
        height: u32,
        source_length: usize,
        is_compressed: bool,
        bpp: u32,
        mipmap_count: u32,
        layer_count: u32,
    ) -> usize {
        swizzle_length_3d(
            width,
            height,
            1,
            source_length,
            is_compressed,
            bpp,
            mipmap_count,
            layer_count,
        )
    }

    fn deswizzle_length(
        width: u32,
        height: u32,
        source_length: usize,
        is_compressed: bool,
        bpp: u32,
        mipmap_count: u32,
        layer_count: u32,
    ) -> usize {
        deswizzle_length_3d(
            width,
            height,
            1,
            source_length,
            is_compressed,
            bpp,
            mipmap_count,
            layer_count,
        )
    }

    fn swizzle_length_3d(
        width: u32,
        height: u32,
        depth: u32,
        source_length: usize,
        is_compressed: bool,
        bpp: u32,
        mipmap_count: u32,
        layer_count: u32,
    ) -> usize {
        swizzle_surface(
            width,
            height,
            depth,
            &vec![0u8; source_length],
            if is_compressed {
                BlockDim::block_4x4()
            } else {
                BlockDim::uncompressed()
            },
            None,
            bpp,
            mipmap_count,
            layer_count,
        )
        .unwrap()
        .len()
    }

    fn deswizzle_length_3d(
        width: u32,
        height: u32,
        depth: u32,
        source_length: usize,
        is_compressed: bool,
        bpp: u32,
        mipmap_count: u32,
        layer_count: u32,
    ) -> usize {
        deswizzle_surface(
            width,
            height,
            depth,
            &vec![0u8; source_length],
            if is_compressed {
                BlockDim::block_4x4()
            } else {
                BlockDim::uncompressed()
            },
            None,
            bpp,
            mipmap_count,
            layer_count,
        )
        .unwrap()
        .len()
    }

    // Expected swizzled sizes are taken from the nutexb footer.
    // Expected deswizzled sizes are the product of the mipmap size sum and the layer count.
    // TODO: Calculate more accurate deswizzled sizes?
    // TODO: Add a CSV of nutexb sizes.
    // TODO: Clean up the existing documentation/data dumps.
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
    fn swizzle_surface_3d_length() {
        assert_eq!(
            16384,
            swizzle_length_3d(16, 16, 16, 16 * 16 * 16 * 4, false, 4, 1, 1)
        );
        assert_eq!(
            368640,
            swizzle_length_3d(33, 33, 33, 33 * 33 * 33 * 4, false, 4, 1, 1)
        );
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
    fn swizzle_surface_potential_overflow_length() {
        assert_eq!(0, swizzle_length_3d(u32::MAX, 0, 0, 0, false, 4, 1, 1));
        assert_eq!(0, swizzle_length_3d(0, u32::MAX, 0, 0, false, 4, 1, 1));
        assert_eq!(0, swizzle_length_3d(0, 0, u32::MAX, 0, false, 4, 1, 1));
        assert_eq!(
            0,
            swizzle_length_3d(u32::MAX, u32::MAX, u32::MAX, 0, false, 0, 1, 1)
        );
        assert_eq!(
            0,
            swizzle_length_3d(u32::MAX, u32::MAX, u32::MAX, 0, false, 1, 0, 1)
        );
        assert_eq!(
            0,
            swizzle_length_3d(u32::MAX, u32::MAX, u32::MAX, 0, false, 1, 1, 0)
        );
    }

    #[test]
    fn deswizzle_surface_nutexb_length() {
        // Sizes and parameters taken from Smash Ultimate nutexb files.
        // The deswizzled size is estimated as the product of the mip sizes sum and layer count.
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

    #[test]
    fn deswizzle_surface_potential_overflow_length() {
        assert_eq!(0, deswizzle_length(u32::MAX, 0, 0, false, 4, 1, 6));
        assert_eq!(0, deswizzle_length(0, u32::MAX, 0, false, 4, 1, 6));
        assert_eq!(0, deswizzle_length(u32::MAX, u32::MAX, 0, false, 0, 1, 6));
        assert_eq!(0, deswizzle_length(u32::MAX, u32::MAX, 0, false, 4, 0, 6));
        assert_eq!(0, deswizzle_length(u32::MAX, u32::MAX, 0, false, 4, 1, 0));
    }

    #[test]
    fn swizzle_surface_not_enough_data() {
        let input = [0, 0, 0, 0];
        let result = swizzle_surface(16, 16, 16, &input, BlockDim::uncompressed(), None, 4, 1, 1);
        assert_eq!(
            result,
            Err(SwizzleError::NotEnoughData {
                expected_size: 16384,
                actual_size: 4
            })
        );
    }

    #[test]
    fn deswizzle_surface_not_enough_data() {
        let input = [0, 0, 0, 0];
        let result = deswizzle_surface(4, 4, 1, &input, BlockDim::uncompressed(), None, 4, 1, 1);
        assert_eq!(
            result,
            Err(SwizzleError::NotEnoughData {
                expected_size: 512,
                actual_size: 4
            })
        );
    }

    #[test]
    fn swizzle_surface_potential_out_of_memory() {
        // Test a large 3D texture that likely won't fit in memory.
        // The input is clearly too small, so this should error instead of panic.
        let input = [0, 0, 0, 0];
        let result = swizzle_surface(
            65535,
            65535,
            65535,
            &input,
            BlockDim::uncompressed(),
            None,
            4,
            1,
            1,
        );
        assert_eq!(
            result,
            Err(SwizzleError::InvalidSurface {
                width: 65535,
                height: 65535,
                depth: 65535,
                bytes_per_pixel: 4,
                mipmap_count: 1
            })
        );
    }

    #[test]
    fn deswizzle_surface_potential_out_of_memory() {
        // Test a large 3D texture that likely won't fit in memory.
        // The input is clearly too small, so this should error instead of panic.
        let input = [0, 0, 0, 0];
        let result = deswizzle_surface(
            65535,
            65535,
            65535,
            &input,
            BlockDim::uncompressed(),
            None,
            4,
            1,
            1,
        );
        assert_eq!(
            result,
            Err(SwizzleError::InvalidSurface {
                width: 65535,
                height: 65535,
                depth: 65535,
                bytes_per_pixel: 4,
                mipmap_count: 1
            })
        );
    }

    #[test]
    fn swizzle_invalid_mipmaps() {
        // A 32-bit integer dimension can only have 32 mipmaps.
        let input = [0; 4];
        let result = swizzle_surface(1, 1, 1, &input, BlockDim::uncompressed(), None, 4, 33, 1);
        assert_eq!(
            result,
            Err(SwizzleError::InvalidSurface {
                width: 1,
                height: 1,
                depth: 1,
                bytes_per_pixel: 4,
                mipmap_count: 33,
            })
        );
    }

    #[test]
    fn deswizzle_surface_invalid_mipmaps() {
        // A 32-bit integer dimension can only have 32 mipmaps.
        let input = [0; 4];
        let result = deswizzle_surface(1, 1, 1, &input, BlockDim::uncompressed(), None, 4, 33, 1);
        assert_eq!(
            result,
            Err(SwizzleError::InvalidSurface {
                width: 1,
                height: 1,
                depth: 1,
                bytes_per_pixel: 4,
                mipmap_count: 33,
            })
        );
    }

    #[test]
    fn swizzle_surface_rgba_16_16_16() {
        let input = include_bytes!("../block_linear/16_16_16_rgba.bin");
        let expected = include_bytes!("../block_linear/16_16_16_rgba_tiled.bin");
        let actual =
            swizzle_surface(16, 16, 16, input, BlockDim::uncompressed(), None, 4, 1, 1).unwrap();
        assert_eq!(expected, &actual[..]);
    }

    #[test]
    fn deswizzle_surface_rgba_16_16_16() {
        let input = include_bytes!("../block_linear/16_16_16_rgba_tiled.bin");
        let expected = include_bytes!("../block_linear/16_16_16_rgba.bin");
        let actual =
            deswizzle_surface(16, 16, 16, input, BlockDim::uncompressed(), None, 4, 1, 1).unwrap();
        assert_eq!(expected, &actual[..]);
    }

    #[test]
    fn swizzle_surface_rgba_33_33_33() {
        let input = include_bytes!("../block_linear/33_33_33_rgba.bin");
        let expected = include_bytes!("../block_linear/33_33_33_rgba_tiled.bin");
        let actual =
            swizzle_surface(33, 33, 33, input, BlockDim::uncompressed(), None, 4, 1, 1).unwrap();
        assert!(expected == &actual[..]);
    }

    #[test]
    fn deswizzle_surface_rgba_33_33_33() {
        let input = include_bytes!("../block_linear/33_33_33_rgba_tiled.bin");
        let expected = include_bytes!("../block_linear/33_33_33_rgba.bin");
        let actual =
            deswizzle_surface(33, 33, 33, input, BlockDim::uncompressed(), None, 4, 1, 1).unwrap();
        assert!(expected == &actual[..]);
    }
}
