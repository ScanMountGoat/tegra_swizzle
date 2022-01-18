// Array alignment code ported from C# implementations of driver code by gdkchan.
// The code can be found here: https://github.com/KillzXGaming/Switch-Toolbox/pull/419#issuecomment-959980096
// This comes from the Ryujinx emulator: https://github.com/Ryujinx/Ryujinx/blob/master/LICENSE.txt.
use crate::{div_round_up, round_up, swizzled_surface_size, BlockHeight};

// TODO: Use consistent conventions with the other code.
// TODO: Should this be part of the public API with an example?
pub fn align_layer_size(
    layer_size: usize,
    height: usize,
    depth: usize,
    // block_height: usize,
    block_height_mip0: BlockHeight,
    depth_in_gobs: usize,
) -> usize {
    // Assume this is 1 based on the github comment linked above.
    // Don't support sparse textures for now.
    let gob_blocks_in_tile_x = 1;

    let mut size = layer_size;
    // let mut height = height;
    let mut gob_height = block_height_mip0 as usize;
    let mut gob_depth = depth_in_gobs;

    if gob_blocks_in_tile_x < 2 {
        // height = div_round_up(height, block_height);

        while height <= (gob_height / 2) * 8 && gob_height > 1 {
            gob_height /= 2;
        }

        while depth <= (gob_depth / 2) && gob_depth > 1 {
            gob_depth /= 2;
        }

        let block_of_gobs_size = gob_height * gob_depth * 512;
        let size_in_block_of_gobs = size / block_of_gobs_size;

        if size != size_in_block_of_gobs * block_of_gobs_size {
            size = (size_in_block_of_gobs + 1) * block_of_gobs_size;
        }
    } else {
        let alignment = (gob_blocks_in_tile_x * 512) * gob_height * gob_depth;

        size = round_up(size, alignment);
    }

    size
}

#[cfg(test)]
mod tests {
    use crate::{block_height_mip0, swizzled_surface_size};

    use super::*;

    fn layer_size_no_mips(
        width: usize,
        height: usize,
        block_width: usize,
        block_height: usize,
        bpp: usize,
    ) -> usize {
        let width = div_round_up(width, block_width);
        let height = div_round_up(height, block_height);
        let block_height_mip0 = block_height_mip0(height);
        let layer_size = swizzled_surface_size(width, height, 1, block_height_mip0, bpp);
        // TODO: Alignment doesn't matter with no mipmaps?
        let aligned = align_layer_size(layer_size, height, 1, block_height_mip0, 1);
        aligned * 6
    }

    // TODO: Add some basic tests for 512 byte alignment.
    // TODO: Find some examples from nutexb files?
    #[test]
    fn layer_size_no_mipmaps() {
        assert_eq!(6144, layer_size_no_mips(16, 16, 1, 1, 4));
        assert_eq!(3072, layer_size_no_mips(16, 16, 4, 4, 8));
        assert_eq!(25165824, layer_size_no_mips(2048, 2048, 4, 4, 16));
        assert_eq!(1572864, layer_size_no_mips(256, 256, 1, 1, 4));
        assert_eq!(98304, layer_size_no_mips(64, 64, 1, 1, 4));
        assert_eq!(98304, layer_size_no_mips(64, 64, 1, 1, 4));
        assert_eq!(393216, layer_size_no_mips(64, 64, 1, 1, 16));
    }

    #[test]
    fn layer_size_mipmaps() {
        // TODO: How to test this?
    }
}
