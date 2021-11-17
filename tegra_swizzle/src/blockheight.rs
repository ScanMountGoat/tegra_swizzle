use crate::{div_round_up, height_in_blocks, BlockHeight};

// Block height code ported from C# implementations of driver code by gdkchan.
// The code can be found here: https://github.com/KillzXGaming/Switch-Toolbox/pull/419#issuecomment-959980096
// This comes from the Ryujinx emulator: https://github.com/Ryujinx/Ryujinx/blob/master/LICENSE.txt.

// TODO: Separate module for this code?
// TODO: Document that this is height in bytes.
/// Calculates the block height parameter to use for the first mip level if no block height is specified.
/// 
/// # Examples
/// Uncompressed formats like R8G8B8A8 can use the height in pixels.
/**
```rust
use tegra_swizzle::{block_height_mip0, mip_block_height};

let height = 300;
let block_height_mip0 = block_height_mip0(height);
```
 */
/// For compressed formats with multiple pixels in a block, divide the height by the block dimensions.
/**
```rust
// BC7 has 4x4 pixel blocks that each take up 16 bytes.
# use tegra_swizzle::{block_height_mip0, mip_block_height};
use tegra_swizzle::{div_round_up};

let height = 300;
let block_height_mip0 = block_height_mip0(div_round_up(height, 4));
```
 */
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
///
/// Uncompressed formats like R8G8B8A8 can use the width and height in pixels.
/// For compressed formats with multiple pixels in a block, divide the width and height by the block dimensions.
/**
```rust
use tegra_swizzle::{block_height_mip0, div_round_up, mip_block_height};

// BC7 has 4x4 pixel blocks that each take up 16 bytes.
let height = 300;
let width = 128;
let mipmap_count = 5;

let block_height_mip0 = block_height_mip0(div_round_up(height, 4));
for mip in 0..mipmap_count {
    let mip_height = div_round_up(height >> mip, 4);

    // The block height will likely change for each mip level.
    let mip_block_height = mip_block_height(mip_height, block_height_mip0);
}
```
 */
pub fn mip_block_height(
    // TODO: It's more consistent with the rest of the API to explicitly pass in the mip height?
    // TODO: Should the other examples change to div_round_up instead of using "/ 4"?
    mip_height: usize,
    block_height_mip0: BlockHeight,
) -> BlockHeight {
    let mut block_height = block_height_mip0 as usize;
    while mip_height <= (block_height / 2) * 8 && block_height > 1 {
        block_height /= 2;
    }

    BlockHeight::new(block_height).unwrap()
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
            mip_block_height(div_round_up(36, 4), block_height_mip0(div_round_up(36, 4)))
        );
        assert_eq!(
            BlockHeight::One,
            mip_block_height(div_round_up(40, 4), block_height_mip0(div_round_up(40, 4)))
        );
        assert_eq!(
            BlockHeight::Two,
            mip_block_height(div_round_up(48, 4), block_height_mip0(div_round_up(48, 4)))
        );
        assert_eq!(
            BlockHeight::Two,
            mip_block_height(div_round_up(48, 4), block_height_mip0(div_round_up(48, 4)))
        );
        assert_eq!(
            BlockHeight::Two,
            mip_block_height(div_round_up(48, 4), block_height_mip0(div_round_up(48, 4)))
        );
        assert_eq!(
            BlockHeight::Two,
            mip_block_height(div_round_up(48, 4), block_height_mip0(div_round_up(48, 4)))
        );
        assert_eq!(
            BlockHeight::Two,
            mip_block_height(div_round_up(64, 4), block_height_mip0(div_round_up(64, 4)))
        );
        assert_eq!(
            BlockHeight::Two,
            mip_block_height(div_round_up(72, 4), block_height_mip0(div_round_up(72, 4)))
        );
        assert_eq!(
            BlockHeight::Two,
            mip_block_height(div_round_up(80, 4), block_height_mip0(div_round_up(80, 4)))
        );
        assert_eq!(
            BlockHeight::Two,
            mip_block_height(div_round_up(80, 4), block_height_mip0(div_round_up(80, 4)))
        );
        assert_eq!(
            BlockHeight::Two,
            mip_block_height(div_round_up(80, 4), block_height_mip0(div_round_up(80, 4)))
        );
        assert_eq!(
            BlockHeight::Two,
            mip_block_height(div_round_up(84, 4), block_height_mip0(div_round_up(84, 4)))
        );
        assert_eq!(
            BlockHeight::Four,
            mip_block_height(div_round_up(96, 4), block_height_mip0(div_round_up(96, 4)))
        );
        assert_eq!(
            BlockHeight::Four,
            mip_block_height(div_round_up(96, 4), block_height_mip0(div_round_up(96, 4)))
        );
        assert_eq!(
            BlockHeight::Four,
            mip_block_height(
                div_round_up(100, 4),
                block_height_mip0(div_round_up(100, 4))
            )
        );
        assert_eq!(
            BlockHeight::Four,
            mip_block_height(
                div_round_up(120, 4),
                block_height_mip0(div_round_up(120, 4))
            )
        );
        assert_eq!(
            BlockHeight::Four,
            mip_block_height(
                div_round_up(124, 4),
                block_height_mip0(div_round_up(124, 4))
            )
        );
        assert_eq!(
            BlockHeight::Four,
            mip_block_height(
                div_round_up(128, 4),
                block_height_mip0(div_round_up(128, 4))
            )
        );
        assert_eq!(
            BlockHeight::Four,
            mip_block_height(
                div_round_up(132, 4),
                block_height_mip0(div_round_up(132, 4))
            )
        );
        assert_eq!(
            BlockHeight::Four,
            mip_block_height(
                div_round_up(140, 4),
                block_height_mip0(div_round_up(140, 4))
            )
        );
        assert_eq!(
            BlockHeight::Four,
            mip_block_height(
                div_round_up(168, 4),
                block_height_mip0(div_round_up(168, 4))
            )
        );
        assert_eq!(
            BlockHeight::Eight,
            mip_block_height(
                div_round_up(176, 4),
                block_height_mip0(div_round_up(176, 4))
            )
        );
        assert_eq!(
            BlockHeight::Eight,
            mip_block_height(
                div_round_up(180, 4),
                block_height_mip0(div_round_up(180, 4))
            )
        );
        assert_eq!(
            BlockHeight::Eight,
            mip_block_height(
                div_round_up(184, 4),
                block_height_mip0(div_round_up(184, 4))
            )
        );
        assert_eq!(
            BlockHeight::Eight,
            mip_block_height(
                div_round_up(192, 4),
                block_height_mip0(div_round_up(192, 4))
            )
        );
        assert_eq!(
            BlockHeight::Eight,
            mip_block_height(
                div_round_up(200, 4),
                block_height_mip0(div_round_up(200, 4))
            )
        );
        assert_eq!(
            BlockHeight::Eight,
            mip_block_height(
                div_round_up(220, 4),
                block_height_mip0(div_round_up(220, 4))
            )
        );
        assert_eq!(
            BlockHeight::Eight,
            mip_block_height(
                div_round_up(256, 4),
                block_height_mip0(div_round_up(256, 4))
            )
        );
        assert_eq!(
            BlockHeight::Eight,
            mip_block_height(
                div_round_up(260, 4),
                block_height_mip0(div_round_up(260, 4))
            )
        );
        assert_eq!(
            BlockHeight::Eight,
            mip_block_height(
                div_round_up(292, 4),
                block_height_mip0(div_round_up(292, 4))
            )
        );
        assert_eq!(
            BlockHeight::Eight,
            mip_block_height(
                div_round_up(300, 4),
                block_height_mip0(div_round_up(300, 4))
            )
        );
        assert_eq!(
            BlockHeight::Eight,
            mip_block_height(
                div_round_up(300, 4),
                block_height_mip0(div_round_up(300, 4))
            )
        );
        assert_eq!(
            BlockHeight::Eight,
            mip_block_height(
                div_round_up(320, 4),
                block_height_mip0(div_round_up(320, 4))
            )
        );
        assert_eq!(
            BlockHeight::Eight,
            mip_block_height(
                div_round_up(320, 4),
                block_height_mip0(div_round_up(320, 4))
            )
        );
        assert_eq!(
            BlockHeight::Eight,
            mip_block_height(
                div_round_up(340, 4),
                block_height_mip0(div_round_up(340, 4))
            )
        );
        assert_eq!(
            BlockHeight::Sixteen,
            mip_block_height(
                div_round_up(360, 4),
                block_height_mip0(div_round_up(360, 4))
            )
        );
        assert_eq!(
            BlockHeight::Sixteen,
            mip_block_height(
                div_round_up(384, 4),
                block_height_mip0(div_round_up(384, 4))
            )
        );
        assert_eq!(
            BlockHeight::Sixteen,
            mip_block_height(
                div_round_up(400, 4),
                block_height_mip0(div_round_up(400, 4))
            )
        );
        assert_eq!(
            BlockHeight::Sixteen,
            mip_block_height(
                div_round_up(500, 4),
                block_height_mip0(div_round_up(500, 4))
            )
        );
        assert_eq!(
            BlockHeight::Sixteen,
            mip_block_height(
                div_round_up(560, 4),
                block_height_mip0(div_round_up(560, 4))
            )
        );
        assert_eq!(
            BlockHeight::Sixteen,
            mip_block_height(
                div_round_up(640, 4),
                block_height_mip0(div_round_up(640, 4))
            )
        );
        assert_eq!(
            BlockHeight::Sixteen,
            mip_block_height(
                div_round_up(720, 4),
                block_height_mip0(div_round_up(720, 4))
            )
        );
        assert_eq!(
            BlockHeight::Sixteen,
            mip_block_height(
                div_round_up(768, 4),
                block_height_mip0(div_round_up(768, 4))
            )
        );
        assert_eq!(
            BlockHeight::Sixteen,
            mip_block_height(
                div_round_up(1088, 4),
                block_height_mip0(div_round_up(1088, 4))
            )
        );
        assert_eq!(
            BlockHeight::Sixteen,
            mip_block_height(
                div_round_up(1152, 4),
                block_height_mip0(div_round_up(1152, 4))
            )
        );
        assert_eq!(
            BlockHeight::Sixteen,
            mip_block_height(
                div_round_up(1408, 4),
                block_height_mip0(div_round_up(1408, 4))
            )
        );
        assert_eq!(
            BlockHeight::One,
            mip_block_height(div_round_up(24, 4), block_height_mip0(div_round_up(48, 4)))
        );
        assert_eq!(
            BlockHeight::One,
            mip_block_height(div_round_up(24, 4), block_height_mip0(div_round_up(48, 4)))
        );
        assert_eq!(
            BlockHeight::One,
            mip_block_height(div_round_up(24, 4), block_height_mip0(div_round_up(48, 4)))
        );
        assert_eq!(
            BlockHeight::One,
            mip_block_height(div_round_up(24, 4), block_height_mip0(div_round_up(48, 4)))
        );
        assert_eq!(
            BlockHeight::One,
            mip_block_height(div_round_up(32, 4), block_height_mip0(div_round_up(64, 4)))
        );
        assert_eq!(
            BlockHeight::Two,
            mip_block_height(div_round_up(36, 4), block_height_mip0(div_round_up(72, 4)))
        );
        assert_eq!(
            BlockHeight::Two,
            mip_block_height(div_round_up(40, 4), block_height_mip0(div_round_up(80, 4)))
        );
        assert_eq!(
            BlockHeight::Two,
            mip_block_height(div_round_up(40, 4), block_height_mip0(div_round_up(80, 4)))
        );
        assert_eq!(
            BlockHeight::Two,
            mip_block_height(div_round_up(42, 4), block_height_mip0(div_round_up(84, 4)))
        );
        assert_eq!(
            BlockHeight::Two,
            mip_block_height(div_round_up(48, 4), block_height_mip0(div_round_up(96, 4)))
        );
        assert_eq!(
            BlockHeight::Two,
            mip_block_height(div_round_up(50, 4), block_height_mip0(div_round_up(100, 4)))
        );
        assert_eq!(
            BlockHeight::Two,
            mip_block_height(div_round_up(64, 4), block_height_mip0(div_round_up(128, 4)))
        );
        assert_eq!(
            BlockHeight::Four,
            mip_block_height(div_round_up(70, 4), block_height_mip0(div_round_up(140, 4)))
        );
        assert_eq!(
            BlockHeight::Four,
            mip_block_height(div_round_up(84, 4), block_height_mip0(div_round_up(168, 4)))
        );
        assert_eq!(
            BlockHeight::Four,
            mip_block_height(div_round_up(90, 4), block_height_mip0(div_round_up(180, 4)))
        );
        assert_eq!(
            BlockHeight::Four,
            mip_block_height(div_round_up(92, 4), block_height_mip0(div_round_up(184, 4)))
        );
        assert_eq!(
            BlockHeight::Four,
            mip_block_height(div_round_up(96, 4), block_height_mip0(div_round_up(192, 4)))
        );
        assert_eq!(
            BlockHeight::Four,
            mip_block_height(
                div_round_up(100, 4),
                block_height_mip0(div_round_up(200, 4))
            )
        );
        assert_eq!(
            BlockHeight::Four,
            mip_block_height(
                div_round_up(110, 4),
                block_height_mip0(div_round_up(220, 4))
            )
        );
        assert_eq!(
            BlockHeight::Four,
            mip_block_height(
                div_round_up(128, 4),
                block_height_mip0(div_round_up(256, 4))
            )
        );
        assert_eq!(
            BlockHeight::Eight,
            mip_block_height(
                div_round_up(130, 4),
                block_height_mip0(div_round_up(260, 4))
            )
        );
        assert_eq!(
            BlockHeight::Eight,
            mip_block_height(
                div_round_up(150, 4),
                block_height_mip0(div_round_up(300, 4))
            )
        );
        assert_eq!(
            BlockHeight::Eight,
            mip_block_height(
                div_round_up(160, 4),
                block_height_mip0(div_round_up(320, 4))
            )
        );
        assert_eq!(
            BlockHeight::Eight,
            mip_block_height(
                div_round_up(160, 4),
                block_height_mip0(div_round_up(320, 4))
            )
        );
        assert_eq!(
            BlockHeight::Eight,
            mip_block_height(
                div_round_up(180, 4),
                block_height_mip0(div_round_up(360, 4))
            )
        );
        assert_eq!(
            BlockHeight::Eight,
            mip_block_height(
                div_round_up(192, 4),
                block_height_mip0(div_round_up(384, 4))
            )
        );
        assert_eq!(
            BlockHeight::Eight,
            mip_block_height(
                div_round_up(200, 4),
                block_height_mip0(div_round_up(400, 4))
            )
        );
        assert_eq!(
            BlockHeight::Eight,
            mip_block_height(
                div_round_up(250, 4),
                block_height_mip0(div_round_up(500, 4))
            )
        );
        assert_eq!(
            BlockHeight::Sixteen,
            mip_block_height(
                div_round_up(280, 4),
                block_height_mip0(div_round_up(560, 4))
            )
        );
        assert_eq!(
            BlockHeight::Sixteen,
            mip_block_height(
                div_round_up(320, 4),
                block_height_mip0(div_round_up(640, 4))
            )
        );
        assert_eq!(
            BlockHeight::Sixteen,
            mip_block_height(
                div_round_up(360, 4),
                block_height_mip0(div_round_up(720, 4))
            )
        );
        assert_eq!(
            BlockHeight::Sixteen,
            mip_block_height(
                div_round_up(384, 4),
                block_height_mip0(div_round_up(768, 4))
            )
        );
        assert_eq!(
            BlockHeight::Sixteen,
            mip_block_height(
                div_round_up(544, 4),
                block_height_mip0(div_round_up(1088, 4))
            )
        );
        assert_eq!(
            BlockHeight::Sixteen,
            mip_block_height(
                div_round_up(576, 4),
                block_height_mip0(div_round_up(1152, 4))
            )
        );
        assert_eq!(
            BlockHeight::Sixteen,
            mip_block_height(
                div_round_up(704, 4),
                block_height_mip0(div_round_up(1408, 4))
            )
        );
        assert_eq!(
            BlockHeight::One,
            mip_block_height(div_round_up(25, 4), block_height_mip0(div_round_up(100, 4)))
        );
        assert_eq!(
            BlockHeight::Two,
            mip_block_height(div_round_up(35, 4), block_height_mip0(div_round_up(140, 4)))
        );
        assert_eq!(
            BlockHeight::Four,
            mip_block_height(div_round_up(75, 4), block_height_mip0(div_round_up(300, 4)))
        );
        assert_eq!(
            BlockHeight::Four,
            mip_block_height(div_round_up(80, 4), block_height_mip0(div_round_up(320, 4)))
        );
        assert_eq!(
            BlockHeight::Four,
            mip_block_height(div_round_up(90, 4), block_height_mip0(div_round_up(360, 4)))
        );
        assert_eq!(
            BlockHeight::Eight,
            mip_block_height(
                div_round_up(140, 4),
                block_height_mip0(div_round_up(560, 4))
            )
        );
        assert_eq!(
            BlockHeight::Eight,
            mip_block_height(
                div_round_up(160, 4),
                block_height_mip0(div_round_up(640, 4))
            )
        );
        assert_eq!(
            BlockHeight::Eight,
            mip_block_height(
                div_round_up(180, 4),
                block_height_mip0(div_round_up(720, 4))
            )
        );
        assert_eq!(
            BlockHeight::Eight,
            mip_block_height(
                div_round_up(192, 4),
                block_height_mip0(div_round_up(768, 4))
            )
        );
        assert_eq!(
            BlockHeight::Sixteen,
            mip_block_height(
                div_round_up(272, 4),
                block_height_mip0(div_round_up(1088, 4))
            )
        );
        assert_eq!(
            BlockHeight::Sixteen,
            mip_block_height(
                div_round_up(288, 4),
                block_height_mip0(div_round_up(1152, 4))
            )
        );
        assert_eq!(
            BlockHeight::Sixteen,
            mip_block_height(
                div_round_up(352, 4),
                block_height_mip0(div_round_up(1408, 4))
            )
        );
        assert_eq!(
            BlockHeight::One,
            mip_block_height(div_round_up(12, 4), block_height_mip0(div_round_up(100, 4)))
        );
        assert_eq!(
            BlockHeight::One,
            mip_block_height(div_round_up(17, 4), block_height_mip0(div_round_up(140, 4)))
        );
        assert_eq!(
            BlockHeight::Two,
            mip_block_height(div_round_up(37, 4), block_height_mip0(div_round_up(300, 4)))
        );
        assert_eq!(
            BlockHeight::Two,
            mip_block_height(div_round_up(40, 4), block_height_mip0(div_round_up(320, 4)))
        );
        assert_eq!(
            BlockHeight::Two,
            mip_block_height(div_round_up(45, 4), block_height_mip0(div_round_up(360, 4)))
        );
        assert_eq!(
            BlockHeight::Four,
            mip_block_height(div_round_up(70, 4), block_height_mip0(div_round_up(560, 4)))
        );
        assert_eq!(
            BlockHeight::Four,
            mip_block_height(div_round_up(80, 4), block_height_mip0(div_round_up(640, 4)))
        );
        assert_eq!(
            BlockHeight::Four,
            mip_block_height(div_round_up(90, 4), block_height_mip0(div_round_up(720, 4)))
        );
        assert_eq!(
            BlockHeight::Four,
            mip_block_height(div_round_up(96, 4), block_height_mip0(div_round_up(768, 4)))
        );
        assert_eq!(
            BlockHeight::Eight,
            mip_block_height(
                div_round_up(136, 4),
                block_height_mip0(div_round_up(1088, 4))
            )
        );
        assert_eq!(
            BlockHeight::Eight,
            mip_block_height(
                div_round_up(144, 4),
                block_height_mip0(div_round_up(1152, 4))
            )
        );
        assert_eq!(
            BlockHeight::Eight,
            mip_block_height(
                div_round_up(176, 4),
                block_height_mip0(div_round_up(1408, 4))
            )
        );
        assert_eq!(
            BlockHeight::One,
            mip_block_height(div_round_up(18, 4), block_height_mip0(div_round_up(300, 4)))
        );
        assert_eq!(
            BlockHeight::One,
            mip_block_height(div_round_up(20, 4), block_height_mip0(div_round_up(320, 4)))
        );
        assert_eq!(
            BlockHeight::One,
            mip_block_height(div_round_up(22, 4), block_height_mip0(div_round_up(360, 4)))
        );
        assert_eq!(
            BlockHeight::Two,
            mip_block_height(div_round_up(35, 4), block_height_mip0(div_round_up(560, 4)))
        );
        assert_eq!(
            BlockHeight::Two,
            mip_block_height(div_round_up(40, 4), block_height_mip0(div_round_up(640, 4)))
        );
        assert_eq!(
            BlockHeight::Two,
            mip_block_height(div_round_up(45, 4), block_height_mip0(div_round_up(720, 4)))
        );
        assert_eq!(
            BlockHeight::Two,
            mip_block_height(div_round_up(48, 4), block_height_mip0(div_round_up(768, 4)))
        );
        assert_eq!(
            BlockHeight::Four,
            mip_block_height(
                div_round_up(68, 4),
                block_height_mip0(div_round_up(1088, 4))
            )
        );
        assert_eq!(
            BlockHeight::Four,
            mip_block_height(
                div_round_up(72, 4),
                block_height_mip0(div_round_up(1152, 4))
            )
        );
        assert_eq!(
            BlockHeight::Four,
            mip_block_height(
                div_round_up(88, 4),
                block_height_mip0(div_round_up(1408, 4))
            )
        );
        assert_eq!(
            BlockHeight::One,
            mip_block_height(div_round_up(20, 4), block_height_mip0(div_round_up(640, 4)))
        );
    }
}
