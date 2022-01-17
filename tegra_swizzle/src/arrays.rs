// Array alignment code ported from C# implementations of driver code by gdkchan.
// The code can be found here: https://github.com/KillzXGaming/Switch-Toolbox/pull/419#issuecomment-959980096
// This comes from the Ryujinx emulator: https://github.com/Ryujinx/Ryujinx/blob/master/LICENSE.txt.
use crate::{div_round_up, round_up};

// TODO: Use consistent conventions with the other code.
// TODO: Should this be part of the public API with an example?
fn align_layer_size(
    layer_size: usize,
    height: usize,
    depth: usize,
    block_height: usize,
    height_in_gobs: usize,
    depth_in_gobs: usize,
) -> usize {
    // Assume this is 1 based on the github comment linked above.
    // Don't support sparse textures for now.
    let gob_blocks_in_tile_x = 1;

    let mut size = layer_size;
    let mut height = height;
    let mut gob_height = height_in_gobs;
    let mut gob_depth = depth_in_gobs;

    if gob_blocks_in_tile_x < 2 {
        height = div_round_up(height, block_height);

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
    // TODO: Add some basic tests for 512 byte alignment.
    // TODO: Find some examples from nutexb files?
    #[test]
    fn test1() {
        // TODO: How to test this?
    }
}
