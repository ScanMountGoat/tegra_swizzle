pub fn swizzle_experimental<T: Copy>(
    x_mask: i32,
    y_mask: i32,
    width: usize,
    height: usize,
    source: &[T],
    destination: &mut [T],
    deswizzle: bool,
) {
    // The bit masking trick to increment the offset is taken from here:
    // https://fgiesen.wordpress.com/2011/01/17/texture-tiling-and-swizzling/
    // The masks allow "skipping over" certain bits when incrementing.
    let mut offset_x = 0i32;
    let mut offset_y = 0i32;

    let mut dst = 0;
    for _ in 0..height {
        for _ in 0..width {
            // The bit patterns don't overlap, so just sum the offsets.
            let src = (offset_x + offset_y) as usize;

            // Swap the offets for swizzling or deswizzling.
            // TODO: The condition doesn't need to be in the inner loop.
            // TODO: Have an inner function and swap the source/destination arguments in the outer function?
            if deswizzle {
                (&mut destination[dst..dst + 1])
                    .copy_from_slice(&source[src..src + 1]);
            } else {
                (&mut destination[src..src + 1])
                    .copy_from_slice(&source[dst..dst + 1]);
            }

            // Use the 2's complement identity (offset + !mask + 1 == offset - mask).
            offset_x = (offset_x - x_mask) & x_mask;
            dst += 1;
        }
        offset_y = (offset_y - y_mask) & y_mask;
    }
}

pub fn calculate_swizzle_pattern(width: u32, height: u32) -> (u32, u32) {
    let block_count = width * height / 16;

    let y_pattern = swizzle_y(height / 4);
    let x_pattern = !y_pattern & (block_count - 1);
    (x_pattern, y_pattern)
}

fn swizzle_y(height_in_blocks: u32) -> u32 {
    let y = !0 >> (height_in_blocks.leading_zeros() + 1);
    (y & 0x1) | ((y & 0x6) << 1) | ((y & (!0 << 3)) << 2)
}

#[cfg(test)]
mod tests {
    use binread::{BinRead, BinReaderExt};

    use super::*;

    fn read_blocks<T: BinRead>(bytes: &[u8]) -> Vec<T> {
        let mut reader = std::io::Cursor::new(bytes);
        let mut blocks = Vec::new();
        while let Ok(block) = reader.read_le::<T>() {
            blocks.push(block);
        }
        blocks
    }

    #[test]
    fn swizzle_y_bc7() {
        // This takes the width in blocks as input.
        assert_eq!(0b101101, swizzle_y(64 / 4));
        assert_eq!(0b1101101, swizzle_y(128 / 4));
        assert_eq!(0b11101101, swizzle_y(256 / 4));
        assert_eq!(0b111101101, swizzle_y(512 / 4));

    }

    #[test]
    fn bit_pattern_bc7_64_64() {
        assert_eq!(
            (0b00000011010010, 0b00000000101101),
            calculate_swizzle_pattern(64, 64)
        );
    }

    #[test]
    fn bit_pattern_bc7_128_128() {
        assert_eq!(
            (0b00001110010010, 0b00000001101101),
            calculate_swizzle_pattern(128, 128)
        );
    }

    #[test]
    fn bit_pattern_bc7_256_256() {
        assert_eq!(
            (0b00111100010010, 0b00000011101101),
            calculate_swizzle_pattern(256, 256)
        );
    }

    #[test]
    fn bit_pattern_bc7_512_512() {
        assert_eq!(
            (0b11111000010010, 0b00000111101101),
            calculate_swizzle_pattern(512, 512)
        );
    }

    
    #[test]
    fn deswizzle_bc7_128_128() {
        let input: Vec<u128> = read_blocks(include_bytes!("../swizzle_data/128_bc7_linear.bin"));
        let expected: Vec<u128> = read_blocks(include_bytes!("../swizzle_data/128_bc7_linear_deswizzle.bin"));
        let mut actual = vec![0u128; 128 * 128 / 16];

        let (x_mask, y_mask) = calculate_swizzle_pattern(128, 128);
        let x_mask = x_mask as i32;
        let y_mask = y_mask as i32;
        swizzle_experimental(x_mask, y_mask, 128 / 4, 128 / 4, &input, &mut actual, true);

        assert_eq!(expected, actual);
    }

    #[test]
    fn deswizzle_bc7_256_256() {
        let input: Vec<u128> = read_blocks(include_bytes!("../swizzle_data/256_bc7_linear.bin"));
        let expected: Vec<u128> = read_blocks(include_bytes!("../swizzle_data/256_bc7_linear_deswizzle.bin"));
        let mut actual = vec![0u128; 256 * 256 / 16];

        let (x_mask, y_mask) = calculate_swizzle_pattern(256, 256);
        let x_mask = x_mask as i32;
        let y_mask = y_mask as i32;
        swizzle_experimental(x_mask, y_mask, 256 / 4, 256 / 4, &input, &mut actual, true);

        assert_eq!(expected, actual);
    }

    #[test]
    fn deswizzle_bc7_512_512() {
        let input: Vec<u128> = read_blocks(include_bytes!("../swizzle_data/512_bc7_linear.bin"));
        let expected: Vec<u128> = read_blocks(include_bytes!("../swizzle_data/512_bc7_linear_deswizzle.bin"));
        let mut actual = vec![0u128; 512 * 512 / 16];

        let (x_mask, y_mask) = calculate_swizzle_pattern(512, 512);
        let x_mask = x_mask as i32;
        let y_mask = y_mask as i32;
        swizzle_experimental(x_mask, y_mask, 512 / 4, 512 / 4, &input, &mut actual, true);

        assert_eq!(expected, actual);
    }
}
