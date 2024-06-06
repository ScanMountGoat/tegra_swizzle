// Array alignment code ported from C# implementations of driver code by gdkchan.
// The code can be found here: https://github.com/KillzXGaming/Switch-Toolbox/pull/419#issuecomment-959980096
// This comes from the Ryujinx emulator: https://github.com/Ryujinx/Ryujinx/blob/master/LICENSE.txt.
use crate::{BlockHeight, GOB_SIZE_IN_BYTES};

pub fn align_layer_size(
    layer_size: usize,
    height: u32,
    depth: u32,
    block_height_mip0: BlockHeight,
    depth_in_gobs: u32,
) -> usize {
    // Assume this is 1 based on the github comment linked above.
    // Don't support sparse textures for now.
    let gob_blocks_in_tile_x = 1;

    // TODO: Avoid mut here?
    let mut size = layer_size;
    let mut gob_height = block_height_mip0 as u32;
    let mut gob_depth = depth_in_gobs;

    if gob_blocks_in_tile_x < 2 {
        // TODO: What does this do?
        while height <= (gob_height / 2) * 8 && gob_height > 1 {
            gob_height /= 2;
        }

        while depth <= (gob_depth / 2) && gob_depth > 1 {
            gob_depth /= 2;
        }

        let block_of_gobs_size = gob_height * gob_depth * GOB_SIZE_IN_BYTES;
        let size_in_block_of_gobs = size / block_of_gobs_size as usize;

        if size != size_in_block_of_gobs * block_of_gobs_size as usize {
            size = (size_in_block_of_gobs + 1) * block_of_gobs_size as usize;
        }
    } else {
        let alignment = (gob_blocks_in_tile_x * GOB_SIZE_IN_BYTES) * gob_height * gob_depth;

        size = size.next_multiple_of(alignment as usize);
    }

    size
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{block_height_mip0, div_round_up, mip_block_height, swizzle::swizzled_mip_size};
    use core::cmp::max;

    // TODO: Avoid duplicating this code?
    fn aligned_size(
        width: u32,
        height: u32,
        block_width: u32,
        block_height: u32,
        bpp: u32,
        mipmap_count: u32,
    ) -> usize {
        let block_height_mip0 = block_height_mip0(div_round_up(height, block_height));

        let mut layer_size = 0;

        for mip in 0..mipmap_count {
            let mip_width = max(div_round_up(width >> mip, block_width), 1);
            let mip_height = max(div_round_up(height >> mip, block_height), 1);

            // The block height will likely change for each mip level.
            let mip_block_height = mip_block_height(mip_height, block_height_mip0);

            layer_size += swizzled_mip_size(mip_width, mip_height, 1, mip_block_height, bpp);
        }

        // Assume 6 array layers.
        align_layer_size(layer_size, height, 1, block_height_mip0, 1) * 6
    }

    // Expected swizzled sizes are taken from the nutexb footer.
    #[test]
    fn layer_sizes_no_mipmaps() {
        assert_eq!(6144, aligned_size(16, 16, 1, 1, 4, 1));
        assert_eq!(3072, aligned_size(16, 16, 4, 4, 8, 1));
        assert_eq!(25165824, aligned_size(2048, 2048, 4, 4, 16, 1));
        assert_eq!(1572864, aligned_size(256, 256, 1, 1, 4, 1));
        assert_eq!(98304, aligned_size(64, 64, 1, 1, 4, 1));
        assert_eq!(98304, aligned_size(64, 64, 1, 1, 4, 1));
        assert_eq!(393216, aligned_size(64, 64, 1, 1, 16, 1));
    }

    #[test]
    fn layer_sizes_mipmaps() {
        assert_eq!(147456, aligned_size(128, 128, 4, 4, 16, 8));
        assert_eq!(15360, aligned_size(16, 16, 4, 4, 16, 5));
        assert_eq!(540672, aligned_size(256, 256, 4, 4, 16, 9));
        assert_eq!(1204224, aligned_size(288, 288, 4, 4, 16, 9));
        assert_eq!(2113536, aligned_size(512, 512, 4, 4, 16, 10));
        assert_eq!(49152, aligned_size(64, 64, 4, 4, 16, 7));
    }
}
