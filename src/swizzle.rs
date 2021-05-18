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
    // TODO: This only works for powers of two.
    // TODO: Use the code from the Tegra TRM?
    
    // Add the correct number of 1 bits based on the dimensions.
    let y_shift = 5;
    let y_mask = !0u32 >> (height.leading_zeros() + 6);
    let y_pattern = 0b01101 | (y_mask << y_shift);
    let block_count = width * height / 16;

    let y_pattern = y_pattern & (block_count - 1);
    let x_pattern = !y_pattern & (block_count - 1);
    (x_pattern, y_pattern)
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
