// #![no_std]
// TODO: We don't need std since the core crate can provide the necessary memcpy operation.

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

// TODO: Add a function to calculate surface size by rounding up to blocks (1 x block_height gobs).
// This can be used to calculate the needed dimensions so swizzle deswizzle can simply return the output array.
// For FFI, it will be easier to pass in existing memory of the appropriate size.
// Add surface size calculation to FFI?
// TODO: Make this public?
pub fn get_surface_size(width: usize, height: usize, bytes_per_pixel: usize) -> usize {
    let width_in_gobs = get_width_in_gobs(width, bytes_per_pixel);
    let block_height = get_block_height(height);
    // TODO: Make gob width and gob height constants?
    let height_in_blocks = div_round_up(height, block_height * 8);
    width_in_gobs * height_in_blocks * block_height * 512
}

fn get_block_height(height: usize) -> usize {
    // Block height can only have certain values based on the Tegra TRM page 1189 table 79.
    let block_height = div_round_up(height, 8);

    // TODO: Is it correct to find the closest power of two?
    match block_height {
        0..=1 => 1,
        2 => 2,
        3..=4 => 4,
        5..=11 => 8,
        // TODO: The TRM mentions 32 also works?
        _ => 16,
    }
}

fn div_round_up(x: usize, d: usize) -> usize {
    (x + d - 1) / d
}

fn get_width_in_gobs(width: usize, bytes_per_pixel: usize) -> usize {
    div_round_up(width * bytes_per_pixel, 64)
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

    #[no_mangle]
    pub unsafe extern "C" fn get_surface_size(
        width: usize,
        height: usize,
        bytes_per_pixel: usize,
    ) -> usize {
        super::get_surface_size(width, height, bytes_per_pixel)
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

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn width_in_gobs_block16() {
        assert_eq!(20, get_width_in_gobs(320 / 4, 16));
    }

    #[test]
    fn block_heights() {
        assert_eq!(8, get_block_height(64));

        // BCN Tiles.
        assert_eq!(16, get_block_height(768 / 4));
        assert_eq!(16, get_block_height(384 / 4));
        assert_eq!(16, get_block_height(384 / 4));
        assert_eq!(8, get_block_height(320 / 4));
        assert_eq!(4, get_block_height(80 / 4));
    }

    #[test]
    fn surface_sizes_block4() {
        assert_eq!(1048576, get_surface_size(512, 512, 4));
    }

    #[test]
    fn surface_sizes_block16() {
        assert_eq!(163840, get_surface_size(320 / 4, 320 / 4, 16));
        assert_eq!(40960, get_surface_size(160 / 4, 160 / 4, 16));
        assert_eq!(1024, get_surface_size(32 / 4, 32 / 4, 16));
    }
}
