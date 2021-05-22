pub fn swizzle_experimental<F: Fn(u32, u32) -> u32, G: Fn(u32, u32) -> u32>(
    swizzle_x: F,
    swizzle_y: G,
    width: usize,
    height: usize,
    source: &[u8],
    destination: &mut [u8],
    deswizzle: bool,
    bytes_per_copy: usize,
) {
    // The bit masking trick to increment the offset is taken from here:
    // https://fgiesen.wordpress.com/2011/01/17/texture-tiling-and-swizzling/
    // The masks allow "skipping over" certain bits when incrementing.
    let mut offset_x = 0i32;
    let mut offset_y = 0i32;

    // TODO: Is the cast to i32 always safe?
    let x_mask = swizzle_x(width as u32, height as u32) as i32;
    let y_mask = swizzle_y(width as u32, height as u32) as i32;

    let mut dst = 0;
    // TODO: This works for 3d textures as well by iterating over depth in the outermost loop.
    for _ in 0..height {
        for _ in 0..width {
            // The bit patterns don't overlap, so just sum the offsets.
            let src = (offset_x + offset_y) as usize;

            // Swap the offets for swizzling or deswizzling.
            // TODO: The condition doesn't need to be in the inner loop.
            // TODO: Have an inner function and swap the source/destination arguments in the outer function?
            if deswizzle {
                (&mut destination[dst..dst + bytes_per_copy])
                    .copy_from_slice(&source[src..src + bytes_per_copy]);
            } else {
                (&mut destination[src..src + bytes_per_copy])
                    .copy_from_slice(&source[dst..dst + bytes_per_copy]);
            }

            // Use the 2's complement identity (offset + !mask + 1 == offset - mask).
            offset_x = (offset_x - x_mask) & x_mask;
            dst += bytes_per_copy;
        }
        offset_y = (offset_y - y_mask) & y_mask;
    }
}

pub fn swizzle_x_bc7(width_in_blocks: u32, height_in_blocks: u32) -> u32 {
    // Left shift by 16 bytes since tiles are 16 bytes.
    if width_in_blocks <= 2 {
        return 0b1 << 4;
    }

    let x = !0 >> (width_in_blocks.leading_zeros() + 1);
    let max_shift = std::cmp::min(32 - height_in_blocks.leading_zeros() - 1, 7);
    let result = ((x & 0x1) << 1)
        | ((x & 0x2) << 3)
        | ((x & (!0 << 2)) << max_shift);
    result << 4
}

pub fn swizzle_y_bc7(_width_in_blocks: u32, height_in_blocks: u32) -> u32 {
    // Left shift by 16 bytes since tiles are 16 bytes.
    if height_in_blocks <= 2 {
        return 0b10 << 4;
    }

    // TODO: This only works up to 256x256 blocks.
    let y = !0 >> (height_in_blocks.leading_zeros() + 1);
    let result = (y & 0x1) | ((y & 0x6) << 1) | ((y & 0x78) << 2) | ((y & 0x80) << 8);
    result << 4
}

pub fn swizzle_x_bc1(width_in_blocks: u32, height_in_blocks: u32) -> u32 {
    let x = !0 >> (width_in_blocks.leading_zeros() + 1);
    (x & 0x1)
        | ((x & 0x2) << 1)
        | ((x & 0x4) << 3)
        | ((x & (!0 << 3)) << (32 - height_in_blocks.leading_zeros() - 1))
}

pub fn swizzle_y_bc1(width_in_blocks: u32, height_in_blocks: u32) -> u32 {
    if height_in_blocks <= 2 {
        return 0b100;
    }

    // TODO: ???
    let y = !0 >> (height_in_blocks.leading_zeros() + 1);
    y
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swizzle_x_bc7_power2() {
        // TODO: Investigate sizes smaller than 16x16.

        // These are left shifted by 4 since tiles are 16 bytes.
        let test_swizzle = |a, b| assert_eq!(a, b, "{:b} != {:b}", a, b);
        test_swizzle(0b10000, swizzle_x_bc7(8 / 4, 8 / 4));
        test_swizzle(0b100100000, swizzle_x_bc7(16 / 4, 16 / 4));
        test_swizzle(0b1100100000, swizzle_x_bc7(32 / 4, 32 / 4));
        test_swizzle(0b110100100000, swizzle_x_bc7(64 / 4, 64 / 4));
        test_swizzle(0b11100100100000, swizzle_x_bc7(128 / 4, 128 / 4));
        test_swizzle(0b1111000100100000, swizzle_x_bc7(256 / 4, 256 / 4));
        test_swizzle(0b111110000100100000, swizzle_x_bc7(512 / 4, 512 / 4));
        test_swizzle(0b1111110000100100000, swizzle_x_bc7(1024 / 4, 1024 / 4));
    }

    #[test]
    fn swizzle_y_bc7_power2() {
        // TODO: Investigate sizes smaller than 16x16.
        // These are left shifted by 4 since tiles are 16 bytes.
        let test_swizzle = |a, b| assert_eq!(a, b, "{:b} != {:b}", a, b);
        test_swizzle(0b100000, swizzle_y_bc7(8 / 4, 8 / 4));
        test_swizzle(0b1010000, swizzle_y_bc7(16 / 4, 16 / 4));
        test_swizzle(0b11010000, swizzle_y_bc7(32 / 4, 32 / 4));
        test_swizzle(0b1011010000, swizzle_y_bc7(64 / 4, 64 / 4));
        test_swizzle(0b11011010000, swizzle_y_bc7(128 / 4, 128 / 4));
        test_swizzle(0b111011010000, swizzle_y_bc7(256 / 4, 256 / 4));
        test_swizzle(0b1111011010000, swizzle_y_bc7(512 / 4, 512 / 4));
        test_swizzle(0b10000001111011010000, swizzle_y_bc7(1024 / 4, 1024 / 4));
    }

    #[test]
    fn swizzle_x_bc1_power2() {
        // TODO: Investigate sizes smaller than 16x16.

        // This takes the width/height in blocks as input.
        let test_swizzle = |a, b| assert_eq!(a, b, "{:b} != {:b}", a, b);
        test_swizzle(0b1, swizzle_x_bc1(8 / 4, 8 / 4));
        test_swizzle(0b101, swizzle_x_bc1(16 / 4, 16 / 4));
        test_swizzle(0b100101, swizzle_x_bc1(32 / 4, 32 / 4));
        test_swizzle(0b10100101, swizzle_x_bc1(64 / 4, 64 / 4));
        test_swizzle(0b1100100101, swizzle_x_bc1(128 / 4, 128 / 4));
        test_swizzle(0b111000100101, swizzle_x_bc1(256 / 4, 256 / 4));
        test_swizzle(0b11110000100101, swizzle_x_bc1(512 / 4, 512 / 4));
    }

    #[test]
    fn swizzle_y_bc1_power2() {
        // TODO: Investigate sizes smaller than 16x16.

        let test_swizzle = |a, b| assert_eq!(a, b, "{:b} != {:b}", a, b);
        test_swizzle(0b100, swizzle_y_bc1(8 / 4, 8 / 4));
        test_swizzle(0b100010, swizzle_y_bc1(16 / 4, 16 / 4));
        test_swizzle(0b1001010, swizzle_y_bc1(32 / 4, 32 / 4));
        test_swizzle(0b100011010, swizzle_y_bc1(64 / 4, 64 / 4));
        test_swizzle(0b11011010, swizzle_y_bc1(128 / 4, 128 / 4));
        test_swizzle(0b1000011011010, swizzle_y_bc1(256 / 4, 256 / 4));
        test_swizzle(0b100000111011010, swizzle_y_bc1(512 / 4, 512 / 4));
    }

    #[test]
    fn deswizzle_bc7_64_64() {
        let input = include_bytes!("../swizzle_data/64_bc7_linear.bin");
        let expected = include_bytes!("../swizzle_data/64_bc7_linear_deswizzle.bin");
        let mut actual = vec![0u8; 64 * 64];

        swizzle_experimental(
            swizzle_x_bc7,
            swizzle_y_bc7,
            64 / 4,
            64 / 4,
            input,
            &mut actual,
            true,
            16,
        );

        assert_eq!(expected, &actual[..]);
    }

    #[test]
    fn deswizzle_bc7_128_128() {
        let input = include_bytes!("../swizzle_data/128_bc7_linear.bin");
        let expected = include_bytes!("../swizzle_data/128_bc7_linear_deswizzle.bin");
        let mut actual = vec![0u8; 128 * 128];

        swizzle_experimental(
            swizzle_x_bc7,
            swizzle_y_bc7,
            128 / 4,
            128 / 4,
            input,
            &mut actual,
            true,
            16,
        );

        assert_eq!(expected, &actual[..]);
    }

    #[test]
    fn deswizzle_bc7_256_256() {
        let input = include_bytes!("../swizzle_data/256_bc7_linear.bin");
        let expected = include_bytes!("../swizzle_data/256_bc7_linear_deswizzle.bin");
        let mut actual = vec![0u8; 256 * 256];

        swizzle_experimental(
            swizzle_x_bc7,
            swizzle_y_bc7,
            256 / 4,
            256 / 4,
            input,
            &mut actual,
            true,
            16,
        );

        assert_eq!(expected, &actual[..]);
    }

    #[test]
    fn deswizzle_bc7_512_512() {
        let input = include_bytes!("../swizzle_data/512_bc7_linear.bin");
        let expected = include_bytes!("../swizzle_data/512_bc7_linear_deswizzle.bin");
        let mut actual = vec![0u8; 512 * 512];

        swizzle_experimental(
            swizzle_x_bc7,
            swizzle_y_bc7,
            512 / 4,
            512 / 4,
            input,
            &mut actual,
            true,
            16,
        );

        assert_eq!(expected, &actual[..]);
    }

    #[test]
    fn deswizzle_bc7_1024_1024() {
        let input = include_bytes!("../swizzle_data/1024_bc7_linear.bin");
        let expected = include_bytes!("../swizzle_data/1024_bc7_linear_deswizzle.bin");
        let mut actual = vec![0u8; 1024 * 1024];

        swizzle_experimental(
            swizzle_x_bc7,
            swizzle_y_bc7,
            1024 / 4,
            1024 / 4,
            input,
            &mut actual,
            true,
            16,
        );

        assert_eq!(expected, &actual[..]);
    }
}
