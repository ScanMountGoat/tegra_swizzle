use crate::{BlockHeight, div_round_up};

// Block height code ported from C# implementations of driver code by gdkchan.
// The code can be found here: https://github.com/KillzXGaming/Switch-Toolbox/pull/419#issuecomment-959980096
// This comes from the Ryujinx emulator: https://github.com/Ryujinx/Ryujinx/blob/master/LICENSE.txt.

// TODO: Separate module for this code?
// TODO: Document that this is height in bytes.
/// Calculates the block height parameter for the first mip level if no block height is specified.
pub fn block_height_mip0(height: usize) -> BlockHeight {
    let height_and_half = height + (height / 2);

    if height_and_half >= 128 {
        BlockHeight::Sixteen
    } else if height_and_half >= 64 {
        BlockHeight::Eight
    } else if height_and_half >= 32 {
        BlockHeight::Four
    } else if height_and_half >= 16 {
        BlockHeight::Two
    } else {
        BlockHeight::One
    }
}

// TODO: Rename these inputs to match the conventions of this library.
// TODO: Rework this example.
/// Calculates the block height parameter for the given mip level.
/// 
/// # Examples
/// For texture formats that don't specify the block height for the base mip level, 
/// use [block_height_mip0] to calculate the initial block height.
/**
```rust
let block_height_mip0 = block_height_mip0(4);
let mipmap_count = 10;
for mip in 0..mipmap_count {
    let mip_block_height = mip_block_height(4, 4, block_height_mip0, mip);
}
```
 */
pub fn mip_block_height(
    height_mip0: usize,
    block_height: usize,
    block_height_mip0: BlockHeight,
    level: usize,
) -> BlockHeight {
    let level_height = std::cmp::max(1, height_mip0 >> level);
    let height_in_blocks = div_round_up(level_height, block_height);

    let mut gob_height = block_height_mip0 as usize;
    while height_in_blocks <= (gob_height / 2) * 8 && gob_height > 1 {
        gob_height /= 2;
    }

    BlockHeight::new(gob_height).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_heights_mip0_bcn() {
        // This test data is based on nutexb textures in Smash Ultimate.
        // Expected block heights were determined manually.
        assert_eq!(BlockHeight::One, block_height_mip0(36 / 4));
        assert_eq!(BlockHeight::One, block_height_mip0(40 / 4));
        assert_eq!(BlockHeight::Two, block_height_mip0(48 / 4));
        assert_eq!(BlockHeight::Two, block_height_mip0(48 / 4));
        assert_eq!(BlockHeight::Two, block_height_mip0(48 / 4));
        assert_eq!(BlockHeight::Two, block_height_mip0(48 / 4));
        assert_eq!(BlockHeight::Two, block_height_mip0(64 / 4));
        assert_eq!(BlockHeight::Two, block_height_mip0(72 / 4));
        assert_eq!(BlockHeight::Two, block_height_mip0(80 / 4));
        assert_eq!(BlockHeight::Two, block_height_mip0(80 / 4));
        assert_eq!(BlockHeight::Two, block_height_mip0(80 / 4));
        assert_eq!(BlockHeight::Two, block_height_mip0(84 / 4));
        assert_eq!(BlockHeight::Four, block_height_mip0(96 / 4));
        assert_eq!(BlockHeight::Four, block_height_mip0(96 / 4));
        assert_eq!(BlockHeight::Four, block_height_mip0(100 / 4));
        assert_eq!(BlockHeight::Four, block_height_mip0(120 / 4));
        assert_eq!(BlockHeight::Four, block_height_mip0(124 / 4));
        assert_eq!(BlockHeight::Four, block_height_mip0(128 / 4));
        assert_eq!(BlockHeight::Four, block_height_mip0(132 / 4));
        assert_eq!(BlockHeight::Four, block_height_mip0(140 / 4));
        assert_eq!(BlockHeight::Four, block_height_mip0(168 / 4));
        assert_eq!(BlockHeight::Eight, block_height_mip0(176 / 4));
        assert_eq!(BlockHeight::Eight, block_height_mip0(180 / 4));
        assert_eq!(BlockHeight::Eight, block_height_mip0(184 / 4));
        assert_eq!(BlockHeight::Eight, block_height_mip0(192 / 4));
        assert_eq!(BlockHeight::Eight, block_height_mip0(200 / 4));
        assert_eq!(BlockHeight::Eight, block_height_mip0(220 / 4));
        assert_eq!(BlockHeight::Eight, block_height_mip0(256 / 4));
        assert_eq!(BlockHeight::Eight, block_height_mip0(260 / 4));
        assert_eq!(BlockHeight::Eight, block_height_mip0(292 / 4));
        assert_eq!(BlockHeight::Eight, block_height_mip0(300 / 4));
        assert_eq!(BlockHeight::Eight, block_height_mip0(300 / 4));
        assert_eq!(BlockHeight::Eight, block_height_mip0(320 / 4));
        assert_eq!(BlockHeight::Eight, block_height_mip0(320 / 4));
        assert_eq!(BlockHeight::Eight, block_height_mip0(340 / 4));
        assert_eq!(BlockHeight::Sixteen, block_height_mip0(360 / 4));
        assert_eq!(BlockHeight::Sixteen, block_height_mip0(384 / 4));
        assert_eq!(BlockHeight::Sixteen, block_height_mip0(400 / 4));
        assert_eq!(BlockHeight::Sixteen, block_height_mip0(500 / 4));
        assert_eq!(BlockHeight::Sixteen, block_height_mip0(560 / 4));
        assert_eq!(BlockHeight::Sixteen, block_height_mip0(640 / 4));
        assert_eq!(BlockHeight::Sixteen, block_height_mip0(720 / 4));
        assert_eq!(BlockHeight::Sixteen, block_height_mip0(768 / 4));
        assert_eq!(BlockHeight::Sixteen, block_height_mip0(1088 / 4));
        assert_eq!(BlockHeight::Sixteen, block_height_mip0(1152 / 4));
        assert_eq!(BlockHeight::Sixteen, block_height_mip0(1408 / 4));
    }

    #[test]
    fn mip_block_heights_bcn() {
        // This test data is based on nutexb textures in Smash Ultimate.
        // Expected block heights were determined manually.
        // This overlaps with the test above to ensure mip 0 works as expected.
        assert_eq!(
            BlockHeight::One,
            mip_block_height(36, 4, block_height_mip0(36 / 4), 0)
        );
        assert_eq!(
            BlockHeight::One,
            mip_block_height(40, 4, block_height_mip0(40 / 4), 0)
        );
        assert_eq!(
            BlockHeight::Two,
            mip_block_height(48, 4, block_height_mip0(48 / 4), 0)
        );
        assert_eq!(
            BlockHeight::Two,
            mip_block_height(48, 4, block_height_mip0(48 / 4), 0)
        );
        assert_eq!(
            BlockHeight::Two,
            mip_block_height(48, 4, block_height_mip0(48 / 4), 0)
        );
        assert_eq!(
            BlockHeight::Two,
            mip_block_height(48, 4, block_height_mip0(48 / 4), 0)
        );
        assert_eq!(
            BlockHeight::Two,
            mip_block_height(64, 4, block_height_mip0(64 / 4), 0)
        );
        assert_eq!(
            BlockHeight::Two,
            mip_block_height(72, 4, block_height_mip0(72 / 4), 0)
        );
        assert_eq!(
            BlockHeight::Two,
            mip_block_height(80, 4, block_height_mip0(80 / 4), 0)
        );
        assert_eq!(
            BlockHeight::Two,
            mip_block_height(80, 4, block_height_mip0(80 / 4), 0)
        );
        assert_eq!(
            BlockHeight::Two,
            mip_block_height(80, 4, block_height_mip0(80 / 4), 0)
        );
        assert_eq!(
            BlockHeight::Two,
            mip_block_height(84, 4, block_height_mip0(84 / 4), 0)
        );
        assert_eq!(
            BlockHeight::Four,
            mip_block_height(96, 4, block_height_mip0(96 / 4), 0)
        );
        assert_eq!(
            BlockHeight::Four,
            mip_block_height(96, 4, block_height_mip0(96 / 4), 0)
        );
        assert_eq!(
            BlockHeight::Four,
            mip_block_height(100, 4, block_height_mip0(100 / 4), 0)
        );
        assert_eq!(
            BlockHeight::Four,
            mip_block_height(120, 4, block_height_mip0(120 / 4), 0)
        );
        assert_eq!(
            BlockHeight::Four,
            mip_block_height(124, 4, block_height_mip0(124 / 4), 0)
        );
        assert_eq!(
            BlockHeight::Four,
            mip_block_height(128, 4, block_height_mip0(128 / 4), 0)
        );
        assert_eq!(
            BlockHeight::Four,
            mip_block_height(132, 4, block_height_mip0(132 / 4), 0)
        );
        assert_eq!(
            BlockHeight::Four,
            mip_block_height(140, 4, block_height_mip0(140 / 4), 0)
        );
        assert_eq!(
            BlockHeight::Four,
            mip_block_height(168, 4, block_height_mip0(168 / 4), 0)
        );
        assert_eq!(
            BlockHeight::Eight,
            mip_block_height(176, 4, block_height_mip0(176 / 4), 0)
        );
        assert_eq!(
            BlockHeight::Eight,
            mip_block_height(180, 4, block_height_mip0(180 / 4), 0)
        );
        assert_eq!(
            BlockHeight::Eight,
            mip_block_height(184, 4, block_height_mip0(184 / 4), 0)
        );
        assert_eq!(
            BlockHeight::Eight,
            mip_block_height(192, 4, block_height_mip0(192 / 4), 0)
        );
        assert_eq!(
            BlockHeight::Eight,
            mip_block_height(200, 4, block_height_mip0(200 / 4), 0)
        );
        assert_eq!(
            BlockHeight::Eight,
            mip_block_height(220, 4, block_height_mip0(220 / 4), 0)
        );
        assert_eq!(
            BlockHeight::Eight,
            mip_block_height(256, 4, block_height_mip0(256 / 4), 0)
        );
        assert_eq!(
            BlockHeight::Eight,
            mip_block_height(260, 4, block_height_mip0(260 / 4), 0)
        );
        assert_eq!(
            BlockHeight::Eight,
            mip_block_height(292, 4, block_height_mip0(292 / 4), 0)
        );
        assert_eq!(
            BlockHeight::Eight,
            mip_block_height(300, 4, block_height_mip0(300 / 4), 0)
        );
        assert_eq!(
            BlockHeight::Eight,
            mip_block_height(300, 4, block_height_mip0(300 / 4), 0)
        );
        assert_eq!(
            BlockHeight::Eight,
            mip_block_height(320, 4, block_height_mip0(320 / 4), 0)
        );
        assert_eq!(
            BlockHeight::Eight,
            mip_block_height(320, 4, block_height_mip0(320 / 4), 0)
        );
        assert_eq!(
            BlockHeight::Eight,
            mip_block_height(340, 4, block_height_mip0(340 / 4), 0)
        );
        assert_eq!(
            BlockHeight::Sixteen,
            mip_block_height(360, 4, block_height_mip0(360 / 4), 0)
        );
        assert_eq!(
            BlockHeight::Sixteen,
            mip_block_height(384, 4, block_height_mip0(384 / 4), 0)
        );
        assert_eq!(
            BlockHeight::Sixteen,
            mip_block_height(400, 4, block_height_mip0(400 / 4), 0)
        );
        assert_eq!(
            BlockHeight::Sixteen,
            mip_block_height(500, 4, block_height_mip0(500 / 4), 0)
        );
        assert_eq!(
            BlockHeight::Sixteen,
            mip_block_height(560, 4, block_height_mip0(560 / 4), 0)
        );
        assert_eq!(
            BlockHeight::Sixteen,
            mip_block_height(640, 4, block_height_mip0(640 / 4), 0)
        );
        assert_eq!(
            BlockHeight::Sixteen,
            mip_block_height(720, 4, block_height_mip0(720 / 4), 0)
        );
        assert_eq!(
            BlockHeight::Sixteen,
            mip_block_height(768, 4, block_height_mip0(768 / 4), 0)
        );
        assert_eq!(
            BlockHeight::Sixteen,
            mip_block_height(1088, 4, block_height_mip0(1088 / 4), 0)
        );
        assert_eq!(
            BlockHeight::Sixteen,
            mip_block_height(1152, 4, block_height_mip0(1152 / 4), 0)
        );
        assert_eq!(
            BlockHeight::Sixteen,
            mip_block_height(1408, 4, block_height_mip0(1408 / 4), 0)
        );
        assert_eq!(
            BlockHeight::One,
            mip_block_height(48, 4, block_height_mip0(48 / 4), 1)
        );
        assert_eq!(
            BlockHeight::One,
            mip_block_height(48, 4, block_height_mip0(48 / 4), 1)
        );
        assert_eq!(
            BlockHeight::One,
            mip_block_height(48, 4, block_height_mip0(48 / 4), 1)
        );
        assert_eq!(
            BlockHeight::One,
            mip_block_height(48, 4, block_height_mip0(48 / 4), 1)
        );
        assert_eq!(
            BlockHeight::One,
            mip_block_height(64, 4, block_height_mip0(64 / 4), 1)
        );
        assert_eq!(
            BlockHeight::Two,
            mip_block_height(72, 4, block_height_mip0(72 / 4), 1)
        );
        assert_eq!(
            BlockHeight::Two,
            mip_block_height(80, 4, block_height_mip0(80 / 4), 1)
        );
        assert_eq!(
            BlockHeight::Two,
            mip_block_height(80, 4, block_height_mip0(80 / 4), 1)
        );
        assert_eq!(
            BlockHeight::Two,
            mip_block_height(84, 4, block_height_mip0(84 / 4), 1)
        );
        assert_eq!(
            BlockHeight::Two,
            mip_block_height(96, 4, block_height_mip0(96 / 4), 1)
        );
        assert_eq!(
            BlockHeight::Two,
            mip_block_height(100, 4, block_height_mip0(100 / 4), 1)
        );
        assert_eq!(
            BlockHeight::Two,
            mip_block_height(128, 4, block_height_mip0(128 / 4), 1)
        );
        assert_eq!(
            BlockHeight::Four,
            mip_block_height(140, 4, block_height_mip0(140 / 4), 1)
        );
        assert_eq!(
            BlockHeight::Four,
            mip_block_height(168, 4, block_height_mip0(168 / 4), 1)
        );
        assert_eq!(
            BlockHeight::Four,
            mip_block_height(180, 4, block_height_mip0(180 / 4), 1)
        );
        assert_eq!(
            BlockHeight::Four,
            mip_block_height(184, 4, block_height_mip0(184 / 4), 1)
        );
        assert_eq!(
            BlockHeight::Four,
            mip_block_height(192, 4, block_height_mip0(192 / 4), 1)
        );
        assert_eq!(
            BlockHeight::Four,
            mip_block_height(200, 4, block_height_mip0(200 / 4), 1)
        );
        assert_eq!(
            BlockHeight::Four,
            mip_block_height(220, 4, block_height_mip0(220 / 4), 1)
        );
        assert_eq!(
            BlockHeight::Four,
            mip_block_height(256, 4, block_height_mip0(256 / 4), 1)
        );
        assert_eq!(
            BlockHeight::Eight,
            mip_block_height(260, 4, block_height_mip0(260 / 4), 1)
        );
        assert_eq!(
            BlockHeight::Eight,
            mip_block_height(300, 4, block_height_mip0(300 / 4), 1)
        );
        assert_eq!(
            BlockHeight::Eight,
            mip_block_height(320, 4, block_height_mip0(320 / 4), 1)
        );
        assert_eq!(
            BlockHeight::Eight,
            mip_block_height(320, 4, block_height_mip0(320 / 4), 1)
        );
        assert_eq!(
            BlockHeight::Eight,
            mip_block_height(360, 4, block_height_mip0(360 / 4), 1)
        );
        assert_eq!(
            BlockHeight::Eight,
            mip_block_height(384, 4, block_height_mip0(384 / 4), 1)
        );
        assert_eq!(
            BlockHeight::Eight,
            mip_block_height(400, 4, block_height_mip0(400 / 4), 1)
        );
        assert_eq!(
            BlockHeight::Eight,
            mip_block_height(500, 4, block_height_mip0(500 / 4), 1)
        );
        assert_eq!(
            BlockHeight::Sixteen,
            mip_block_height(560, 4, block_height_mip0(560 / 4), 1)
        );
        assert_eq!(
            BlockHeight::Sixteen,
            mip_block_height(640, 4, block_height_mip0(640 / 4), 1)
        );
        assert_eq!(
            BlockHeight::Sixteen,
            mip_block_height(720, 4, block_height_mip0(720 / 4), 1)
        );
        assert_eq!(
            BlockHeight::Sixteen,
            mip_block_height(768, 4, block_height_mip0(768 / 4), 1)
        );
        assert_eq!(
            BlockHeight::Sixteen,
            mip_block_height(1088, 4, block_height_mip0(1088 / 4), 1)
        );
        assert_eq!(
            BlockHeight::Sixteen,
            mip_block_height(1152, 4, block_height_mip0(1152 / 4), 1)
        );
        assert_eq!(
            BlockHeight::Sixteen,
            mip_block_height(1408, 4, block_height_mip0(1408 / 4), 1)
        );
        assert_eq!(
            BlockHeight::One,
            mip_block_height(100, 4, block_height_mip0(100 / 4), 2)
        );
        assert_eq!(
            BlockHeight::Two,
            mip_block_height(140, 4, block_height_mip0(140 / 4), 2)
        );
        assert_eq!(
            BlockHeight::Four,
            mip_block_height(300, 4, block_height_mip0(300 / 4), 2)
        );
        assert_eq!(
            BlockHeight::Four,
            mip_block_height(320, 4, block_height_mip0(320 / 4), 2)
        );
        assert_eq!(
            BlockHeight::Four,
            mip_block_height(360, 4, block_height_mip0(360 / 4), 2)
        );
        assert_eq!(
            BlockHeight::Eight,
            mip_block_height(560, 4, block_height_mip0(560 / 4), 2)
        );
        assert_eq!(
            BlockHeight::Eight,
            mip_block_height(640, 4, block_height_mip0(640 / 4), 2)
        );
        assert_eq!(
            BlockHeight::Eight,
            mip_block_height(720, 4, block_height_mip0(720 / 4), 2)
        );
        assert_eq!(
            BlockHeight::Eight,
            mip_block_height(768, 4, block_height_mip0(768 / 4), 2)
        );
        assert_eq!(
            BlockHeight::Sixteen,
            mip_block_height(1088, 4, block_height_mip0(1088 / 4), 2)
        );
        assert_eq!(
            BlockHeight::Sixteen,
            mip_block_height(1152, 4, block_height_mip0(1152 / 4), 2)
        );
        assert_eq!(
            BlockHeight::Sixteen,
            mip_block_height(1408, 4, block_height_mip0(1408 / 4), 2)
        );
        assert_eq!(
            BlockHeight::One,
            mip_block_height(100, 4, block_height_mip0(100 / 4), 3)
        );
        assert_eq!(
            BlockHeight::One,
            mip_block_height(140, 4, block_height_mip0(140 / 4), 3)
        );
        assert_eq!(
            BlockHeight::Two,
            mip_block_height(300, 4, block_height_mip0(300 / 4), 3)
        );
        assert_eq!(
            BlockHeight::Two,
            mip_block_height(320, 4, block_height_mip0(320 / 4), 3)
        );
        assert_eq!(
            BlockHeight::Two,
            mip_block_height(360, 4, block_height_mip0(360 / 4), 3)
        );
        assert_eq!(
            BlockHeight::Four,
            mip_block_height(560, 4, block_height_mip0(560 / 4), 3)
        );
        assert_eq!(
            BlockHeight::Four,
            mip_block_height(640, 4, block_height_mip0(640 / 4), 3)
        );
        assert_eq!(
            BlockHeight::Four,
            mip_block_height(720, 4, block_height_mip0(720 / 4), 3)
        );
        assert_eq!(
            BlockHeight::Four,
            mip_block_height(768, 4, block_height_mip0(768 / 4), 3)
        );
        assert_eq!(
            BlockHeight::Eight,
            mip_block_height(1088, 4, block_height_mip0(1088 / 4), 3)
        );
        assert_eq!(
            BlockHeight::Eight,
            mip_block_height(1152, 4, block_height_mip0(1152 / 4), 3)
        );
        assert_eq!(
            BlockHeight::Eight,
            mip_block_height(1408, 4, block_height_mip0(1408 / 4), 3)
        );
        assert_eq!(
            BlockHeight::One,
            mip_block_height(300, 4, block_height_mip0(300 / 4), 4)
        );
        assert_eq!(
            BlockHeight::One,
            mip_block_height(320, 4, block_height_mip0(320 / 4), 4)
        );
        assert_eq!(
            BlockHeight::One,
            mip_block_height(360, 4, block_height_mip0(360 / 4), 4)
        );
        assert_eq!(
            BlockHeight::Two,
            mip_block_height(560, 4, block_height_mip0(560 / 4), 4)
        );
        assert_eq!(
            BlockHeight::Two,
            mip_block_height(640, 4, block_height_mip0(640 / 4), 4)
        );
        assert_eq!(
            BlockHeight::Two,
            mip_block_height(720, 4, block_height_mip0(720 / 4), 4)
        );
        assert_eq!(
            BlockHeight::Two,
            mip_block_height(768, 4, block_height_mip0(768 / 4), 4)
        );
        assert_eq!(
            BlockHeight::Four,
            mip_block_height(1088, 4, block_height_mip0(1088 / 4), 4)
        );
        assert_eq!(
            BlockHeight::Four,
            mip_block_height(1152, 4, block_height_mip0(1152 / 4), 4)
        );
        assert_eq!(
            BlockHeight::Four,
            mip_block_height(1408, 4, block_height_mip0(1408 / 4), 4)
        );
        assert_eq!(
            BlockHeight::One,
            mip_block_height(640, 4, block_height_mip0(640 / 4), 5)
        );
    }
}
