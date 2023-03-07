// Block depth code ported from C# implementations of driver code by gdkchan in Ryujinx.
// The code can be found here: https://github.com/KillzXGaming/Switch-Toolbox/pull/419#issuecomment-959980096
// License MIT: https://github.com/Ryujinx/Ryujinx/blob/master/LICENSE.txt.
pub const fn block_depth(depth: usize) -> usize {
    // TODO: Should this be an enum similar to BlockHeight?
    // This would only matter if it was part of the public API.
    let depth_and_half = depth + (depth / 2);
    if depth_and_half >= 16 {
        16
    } else if depth_and_half >= 8 {
        8
    } else if depth_and_half >= 4 {
        4
    } else if depth_and_half >= 2 {
        2
    } else {
        1
    }
}

pub fn mip_block_depth(mip_depth: usize, gob_depth: usize) -> usize {
    let mut gob_depth = gob_depth;
    while mip_depth <= gob_depth / 2 && gob_depth > 1 {
        gob_depth /= 2;
    }

    gob_depth
}

#[cfg(test)]
mod tests {
    // TODO: Create additional test cases based on existing game assets.
    // 3D textures are rare, so it's hard to find examples for this.
    use super::*;

    #[test]
    fn base_block_depths() {
        assert_eq!(16, block_depth(16));
        assert_eq!(16, block_depth(33));
    }

    #[test]
    fn mip_block_depths() {
        assert_eq!(8, mip_block_depth(16 / 2, 16));
        assert_eq!(16, mip_block_depth(33 / 2, 16));
    }
}
