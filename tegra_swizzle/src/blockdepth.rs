// Block depth code ported from C# implementations of driver code by gdkchan.
// The code can be found here: https://github.com/KillzXGaming/Switch-Toolbox/pull/419#issuecomment-959980096
// This comes from the Ryujinx emulator: https://github.com/Ryujinx/Ryujinx/blob/master/LICENSE.txt.
pub fn block_depth(depth: usize) -> usize {
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

// TODO: Can these be calculated automatically?
// These aren't listed as directly user configurable in the TRM?
pub fn mip_block_depth(base_depth: usize, gob_depth: usize, level: usize) -> usize {
    // TODO: Factor out the mip level to be consistent with blockheight.rs?
    let level_depth = std::cmp::max(1, base_depth >> level);

    let mut gob_depth = gob_depth;
    while level_depth <= gob_depth / 2 && gob_depth > 1 {
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
    fn base_block_depth() {
        assert_eq!(16, block_depth(16));
    }
}
