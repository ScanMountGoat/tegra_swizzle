// #![no_std]
// TODO: We don't need std since the core crate can provide the necessary memcpy operation.

// Width and height are calculated as width/4 and height/4 for BCN compression.
// TODO: Is this even more performant for power of two sizes?
fn swizzle_experimental<F: Fn(u32, u32) -> u32, G: Fn(u32, u32) -> u32>(
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

fn swizzle_x_16(width_in_blocks: u32, height_in_blocks: u32) -> u32 {
    // Left shift by 4 bits since tiles or pixels are 16 bytes.
    if width_in_blocks <= 2 {
        return 0b1 << 4;
    }

    let x = !0 >> (width_in_blocks.leading_zeros() + 1);
    let mut max_shift = 32 - height_in_blocks.leading_zeros() - 1;
    if max_shift > 7 {
        max_shift = 7;
    }
    let result = ((x & 0x1) << 1) | ((x & 0x2) << 3) | ((x & (!0 << 2)) << max_shift);
    result << 4
}

fn swizzle_y_16(_width_in_blocks: u32, height_in_blocks: u32) -> u32 {
    // Left shift by 4 bits since tiles or pixels are 16 bytes.
    if height_in_blocks <= 2 {
        return 0b10 << 4;
    }

    // TODO: This only works up to 256x256.
    let y = !0 >> (height_in_blocks.leading_zeros() + 1);
    let result = (y & 0x1) | ((y & 0x6) << 1) | ((y & 0x78) << 2) | ((y & 0x80) << 8);
    result << 4
}

fn swizzle_x_8(width_in_blocks: u32, height_in_blocks: u32) -> u32 {
    // Left shift by 3 bits since tiles are 8 bytes.
    let x = !0 >> (width_in_blocks.leading_zeros() + 1);
    let result = (x & 0x1)
        | ((x & 0x2) << 1)
        | ((x & 0x4) << 3)
        | ((x & (!0 << 3)) << (32 - height_in_blocks.leading_zeros() - 1));
    result << 3
}

fn swizzle_y_8(_width_in_blocks: u32, height_in_blocks: u32) -> u32 {
    // Left shift by 3 bits since tiles or pixels are 8 bytes.
    // TODO: This only works up to 128x128.
    let y = !0 >> (height_in_blocks.leading_zeros() + 1);
    let result = ((y & 0x1) << 1) | ((y & 0x6) << 2) | ((y & 0x78) << 3);
    result << 3
}

// Code taken from examples in Tegra TRM page 1187.
fn get_gob_address(
    x: usize,
    y: usize,
    block_height: usize,
    image_width_in_gobs: usize,
    bytes_per_pixel: usize,
) -> usize {
    // TODO: Optimize this?
    (y / (8 * block_height)) * 512 * block_height * image_width_in_gobs
        + (x * bytes_per_pixel / 64) * 512 * block_height
        + (y % (8 * block_height) / 8) * 512
}

// Code taken from examples in Tegra TRM page 1188.
// The function has been modified slightly to account for bytes per pixel or compressed tile.
// x, y are byte indices for the 2d pixel grid.
// The returned value is the offset into the gob where the byte is stored.
fn get_gob_offset(x: usize, y: usize) -> usize {
    // TODO: Optimize this to use a lookup table based on x%64 and y%8?
    // TODO: Can a macro generate this lookup?
    ((x % 64) / 32) * 256 + ((y % 8) / 2) * 64 + ((x % 32) / 16) * 32 + (y % 2) * 16 + (x % 16)
}

fn get_address(
    x: usize,
    y: usize,
    block_height: usize,
    image_width_in_gobs: usize,
    bytes_per_pixel: usize,
) -> usize {
    let gob_address = get_gob_address(x, y, block_height, image_width_in_gobs, bytes_per_pixel);
    let gob_offset = get_gob_offset(x * bytes_per_pixel, y);
    gob_address + gob_offset
}

fn get_block_height(height: usize) -> usize {
    // Block height can only have certain values based on the Tegra TRM page 1189 table 79.
    let block_height = height / 8;
    // 0 1 2 3 4 5 6 7 8 9 10 11    12    13 14 15 16

    // TODO: Is it correct to find the closest power of two?
    match block_height {
        0..=1 => 1,
        2 => 2,
        3..=4 => 4,
        5..=12 => 8,
        // TODO: The TRM mentions 32 also works?
        _ => 16,
    }
}

// TODO: Avoid panics?

// TODO: Add another option with a specified block height (use an enum)?
// TODO: Add an option to automatically calculate the output size?
/// Swizzles the bytes in `source` to `destination`.
/// `source` is expected to have at least `width * height * bytes_per_pixel` many bytes.
/// # Panics
/// Panics on out of bounds accesses for `source` or `destination`. This occurs when `source` or `destination` contain too few bytes
/// for the given parameters.
pub fn swizzle_block_linear(
    width: usize,
    height: usize,
    source: &[u8],
    destination: &mut [u8],
    bytes_per_pixel: usize,
) {
    let block_height = get_block_height(height);
    let image_width_in_gobs = get_width_in_gobs(width, bytes_per_pixel);

    // TODO: Extend this to work with depth as well.
    for y in 0..height {
        for x in 0..width {
            // The bit patterns don't overlap, so just sum the offsets.
            let src = get_address(x, y, block_height, image_width_in_gobs, bytes_per_pixel);
            let dst = (y * width + x) * bytes_per_pixel;

            // Swap the offets for swizzling or deswizzling.
            // TODO: The condition doesn't need to be in the inner loop.
            // TODO: Have an inner function and swap the source/destination arguments in the outer function?
            (&mut destination[src..src + bytes_per_pixel])
                .copy_from_slice(&source[dst..dst + bytes_per_pixel]);
        }
    }
}

pub mod ffi {
    #[no_mangle]
    pub unsafe extern "C" fn swizzle_block_linear(
        width: usize,
        height: usize,
        source: *const u8,
        source_len: usize,
        destination: *mut u8,
        destination_len: usize,
        bytes_per_pixel: usize,
    ) {
        let source = std::slice::from_raw_parts(source, source_len);
        let destination = std::slice::from_raw_parts_mut(destination, destination_len);

        super::swizzle_block_linear(width, height, source, destination, bytes_per_pixel)
    }

    #[no_mangle]
    pub unsafe extern "C" fn deswizzle_block_linear(
        width: usize,
        height: usize,
        source: *const u8,
        source_len: usize,
        destination: *mut u8,
        destination_len: usize,
        bytes_per_pixel: usize,
    ) {
        let source = std::slice::from_raw_parts(source, source_len);
        let destination = std::slice::from_raw_parts_mut(destination, destination_len);

        super::deswizzle_block_linear(width, height, source, destination, bytes_per_pixel)
    }
}

// TODO: Add an option to automatically calculate the output size?
/// Deswizzles the bytes in `source` to `destination`.
/// `destination` is expected to have at least `width * height * bytes_per_pixel` many bytes.
/// Swizzling and then deswizzling or deswizzling and then swizzling leaves the input unchanged.
/// # Panics
/// Panics on out of bounds accesses for `source` or `destination`. This occurs when `source` or `destination` contain too few bytes
/// for the given parameters.
pub fn deswizzle_block_linear(
    width: usize,
    height: usize,
    source: &[u8],
    destination: &mut [u8],
    bytes_per_pixel: usize,
) {
    let block_height = get_block_height(height);
    let image_width_in_gobs = get_width_in_gobs(width, bytes_per_pixel);

    // TODO: Extend this to work with depth as well.
    for y in 0..height {
        for x in 0..width {
            // The bit patterns don't overlap, so just sum the offsets.
            let src = get_address(x, y, block_height, image_width_in_gobs, bytes_per_pixel);
            let dst = (y * width + x) * bytes_per_pixel;

            (&mut destination[dst..dst + bytes_per_pixel])
                .copy_from_slice(&source[src..src + bytes_per_pixel]);
        }
    }
}

fn get_width_in_gobs(width: usize, bytes_per_pixel: usize) -> usize {
    // TODO: Round up?
    width * bytes_per_pixel / 64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(test)]
    extern crate std;

    use std::vec;

    #[test]
    #[ignore]
    fn swizzle_x_16_power2() {
        // TODO: Investigate sizes smaller than 16x16.

        // These are left shifted by 4 since tiles are 16 bytes.
        let test_swizzle = |a, b| assert_eq!(a, b, "{:b} != {:b}", a, b);
        test_swizzle(0b10000, swizzle_x_16(8 / 4, 8 / 4));
        test_swizzle(0b100100000, swizzle_x_16(16 / 4, 16 / 4));
        test_swizzle(0b1100100000, swizzle_x_16(32 / 4, 32 / 4));
        test_swizzle(0b110100100000, swizzle_x_16(64 / 4, 64 / 4));
        test_swizzle(0b11100100100000, swizzle_x_16(128 / 4, 128 / 4));
        test_swizzle(0b1111000100100000, swizzle_x_16(256 / 4, 256 / 4));
        test_swizzle(0b111110000100100000, swizzle_x_16(512 / 4, 512 / 4));
        test_swizzle(0b1111110000100100000, swizzle_x_16(1024 / 4, 1024 / 4));
        test_swizzle(0b11111110000100100000, swizzle_x_16(2048 / 4, 2048 / 4));
        // TODO: Fix these test cases.
        test_swizzle(0b111111100000001110000, swizzle_x_16(4096 / 4, 4096 / 4));
    }

    #[test]
    #[ignore]
    fn swizzle_y_16_power2() {
        // TODO: Investigate sizes smaller than 16x16.
        // These are left shifted by 4 since tiles are 16 bytes.
        let test_swizzle = |a, b| assert_eq!(a, b, "{:b} != {:b}", a, b);
        test_swizzle(0b100000, swizzle_y_16(8 / 4, 8 / 4));
        test_swizzle(0b1010000, swizzle_y_16(16 / 4, 16 / 4));
        test_swizzle(0b11010000, swizzle_y_16(32 / 4, 32 / 4));
        test_swizzle(0b1011010000, swizzle_y_16(64 / 4, 64 / 4));
        test_swizzle(0b11011010000, swizzle_y_16(128 / 4, 128 / 4));
        test_swizzle(0b111011010000, swizzle_y_16(256 / 4, 256 / 4));
        test_swizzle(0b1111011010000, swizzle_y_16(512 / 4, 512 / 4));
        test_swizzle(0b10000001111011010000, swizzle_y_16(1024 / 4, 1024 / 4));
        // TODO: Fix these test cases.
        test_swizzle(0b1100000001111011010000, swizzle_x_16(2048 / 4, 2048 / 4));
        test_swizzle(0b111000000011111110000000, swizzle_x_16(4096 / 4, 4096 / 4));
    }

    #[test]
    fn swizzle_x_8_power2() {
        // TODO: Investigate sizes smaller than 16x16.

        // These are left shifted by 3 since tiles are 8 bytes.
        let test_swizzle = |a, b| assert_eq!(a, b, "{:b} != {:b}", a, b);
        test_swizzle(0b1000, swizzle_x_8(8 / 4, 8 / 4));
        test_swizzle(0b101000, swizzle_x_8(16 / 4, 16 / 4));
        test_swizzle(0b100101000, swizzle_x_8(32 / 4, 32 / 4));
        test_swizzle(0b10100101000, swizzle_x_8(64 / 4, 64 / 4));
        test_swizzle(0b1100100101000, swizzle_x_8(128 / 4, 128 / 4));
        test_swizzle(0b111000100101000, swizzle_x_8(256 / 4, 256 / 4));
        test_swizzle(0b11110000100101000, swizzle_x_8(512 / 4, 512 / 4));
    }

    #[test]
    fn swizzle_y_8_power2() {
        // TODO: Investigate sizes smaller than 16x16.

        // These are left shifted by 3 since tiles are 8 bytes.
        let test_swizzle = |a, b| assert_eq!(a, b, "{:b} != {:b}", a, b);
        test_swizzle(0b10000, swizzle_y_8(8 / 4, 8 / 4));
        test_swizzle(0b1010000, swizzle_y_8(16 / 4, 16 / 4));
        test_swizzle(0b11010000, swizzle_y_8(32 / 4, 32 / 4));
        test_swizzle(0b1011010000, swizzle_y_8(64 / 4, 64 / 4));
        test_swizzle(0b11011010000, swizzle_y_8(128 / 4, 128 / 4));
        test_swizzle(0b111011010000, swizzle_y_8(256 / 4, 256 / 4));
        test_swizzle(0b1111011010000, swizzle_y_8(512 / 4, 512 / 4));
    }

    #[test]
    fn deswizzle_bc7_64_64() {
        let input = include_bytes!("../../swizzle_data/64_bc7_linear.bin");
        let expected = include_bytes!("../../swizzle_data/64_bc7_linear_deswizzle.bin");
        let mut actual = vec![0u8; 64 * 64];

        deswizzle_block_linear(64 / 4, 64 / 4, input, &mut actual, 16);

        assert_eq!(expected, &actual[..]);
    }

    #[test]
    fn deswizzle_bc1_128_128() {
        let input = include_bytes!("../../swizzle_data/128_bc1_linear.bin");
        let expected = include_bytes!("../../swizzle_data/128_bc1_linear_deswizzle.bin");
        let mut actual = vec![0u8; 128 * 128 / 16 * 8];

        deswizzle_block_linear(128 / 4, 128 / 4, input, &mut actual, 8);

        assert_eq!(expected, &actual[..]);
    }

    #[test]
    fn deswizzle_bc3_128_128() {
        let input = include_bytes!("../../swizzle_data/128_bc3_linear.bin");
        let expected = include_bytes!("../../swizzle_data/128_bc3_linear_deswizzle.bin");
        let mut actual = vec![0u8; 128 * 128];

        // BC3 has the same swizzle patterns as BC7.
        deswizzle_block_linear(128 / 4, 128 / 4, input, &mut actual, 16);

        assert_eq!(expected, &actual[..]);
    }

    #[test]
    fn deswizzle_rgba_f32_128_128() {
        let input = include_bytes!("../../swizzle_data/128_rgbaf32_linear.bin");
        let expected = include_bytes!("../../swizzle_data/128_rgbaf32_linear_deswizzle.bin");
        let mut actual = vec![0u8; 128 * 128 * 16];

        // R32G32B32A32_FLOAT has the same swizzle patterns as BC7.
        deswizzle_block_linear(128, 128, input, &mut actual, 16);

        assert_eq!(expected, &actual[..]);
    }

    #[test]
    fn deswizzle_bc7_128_128() {
        let input = include_bytes!("../../swizzle_data/128_bc7_linear.bin");
        let expected = include_bytes!("../../swizzle_data/128_bc7_linear_deswizzle.bin");
        let mut actual = vec![0u8; 128 * 128];

        deswizzle_block_linear(128 / 4, 128 / 4, input, &mut actual, 16);

        assert_eq!(expected, &actual[..]);
    }

    #[test]
    fn deswizzle_bc7_256_256() {
        let input = include_bytes!("../../swizzle_data/256_bc7_linear.bin");
        let expected = include_bytes!("../../swizzle_data/256_bc7_linear_deswizzle.bin");
        let mut actual = vec![0u8; 256 * 256];

        deswizzle_block_linear(256 / 4, 256 / 4, input, &mut actual, 16);

        assert_eq!(expected, &actual[..]);
    }

    #[test]
    fn deswizzle_bc7_320_320() {
        let input = include_bytes!("../../swizzle_data/320_bc7_linear.bin");
        let expected = include_bytes!("../../swizzle_data/320_bc7_linear_deswizzle.bin");
        let mut actual = vec![0u8; 320 * 320];

        deswizzle_block_linear(320 / 4, 320 / 4, input, &mut actual, 16);

        assert_eq!(expected, &actual[..]);
    }

    #[test]
    fn deswizzle_bc7_512_512() {
        let input = include_bytes!("../../swizzle_data/512_bc7_linear.bin");
        let expected = include_bytes!("../../swizzle_data/512_bc7_linear_deswizzle.bin");
        let mut actual = vec![0u8; 512 * 512];

        deswizzle_block_linear(512 / 4, 512 / 4, input, &mut actual, 16);

        assert_eq!(expected, &actual[..]);
    }

    #[test]
    fn deswizzle_bc7_1024_1024() {
        let input = include_bytes!("../../swizzle_data/1024_bc7_linear.bin");
        let expected = include_bytes!("../../swizzle_data/1024_bc7_linear_deswizzle.bin");
        let mut actual = vec![0u8; 1024 * 1024];

        deswizzle_block_linear(1024 / 4, 1024 / 4, input, &mut actual, 16);

        assert_eq!(expected, &actual[..]);
    }
}
