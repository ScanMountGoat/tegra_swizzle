//! Functions for swizzling and deswizzling texture data for the Tegra X1's block linear format.
//! Block linear arranges bytes of a texture surface into a 2D grid of blocks.
//! Groups of 512 bytes form GOBs ("group of bytes") where each GOB is 64x8 bytes.
//! The `block_height` parameter determines how many GOBs stack vertically to form a block.
//!
//! Blocks are arranged linearly in row-major order. Each block has a width of 1 GOB and a height of `block_height` many GOBs.
//!
//! Pixels or 4x4 pixel tiles for BCN compressed formats are arranged horizontally to form a row of `width_in_pixels * bytes_per_pixel` many bytes
//! or `width_in_pixels / 4 * bytes_per_tile` many bytes.
//! The surface width is rounded up to the width in blocks or 64 bytes (one GOB).
//!
//! The height of the surface is `height_in_pixels` many bytes for most formats and
//! `height_in_pixels / 4` many bytes for BCN compressed formats since the tiles are arranged horizontally within the row.
//! The surface height is rounded up to the height in blocks or `block_height * 8` bytes.

// #![no_std]
// TODO: We don't need std since the core crate can provide the necessary memcpy operation.

// TODO: Use u32 for everything?
const GOB_WIDTH: usize = 64;
const GOB_HEIGHT: usize = 8;
const GOB_SIZE: usize = GOB_WIDTH * GOB_HEIGHT;

// Code taken from examples in Tegra TRM page 1187.
// Return the starting address of the GOB containing the pixel at location (x, y).
fn gob_address(
    x: usize,
    y: usize,
    block_height: usize,
    image_width_in_gobs: usize,
    bytes_per_pixel: usize,
) -> usize {
    // TODO: Optimize this?
    // TODO: Is this a row major index based on blocks?
    (y / (GOB_HEIGHT * block_height)) * GOB_SIZE * block_height * image_width_in_gobs // block_row * bytes_per_row?
        + (x * bytes_per_pixel / GOB_WIDTH) * GOB_SIZE * block_height // block_column * bytes_per_column?
        + (y % (GOB_HEIGHT * block_height) / GOB_HEIGHT) * GOB_SIZE // find the right column within a block?
}

// Code taken from examples in Tegra TRM page 1188.
// Return the offset within the GOB for the byte at location (x, y).
fn gob_offset(x: usize, y: usize) -> usize {
    // TODO: Optimize this?
    // TODO: Describe the pattern here?
    ((x % 64) / 32) * 256 + ((y % 8) / 2) * 64 + ((x % 32) / 16) * 32 + (y % 2) * 16 + (x % 16)
}

// Given pixel coordinates (x, y), find the offset in the swizzled image data.
// This can be used for swizzling and deswizzling operations.
fn swizzled_address(
    x: usize,
    y: usize,
    block_height: usize,
    image_width_in_gobs: usize,
    bytes_per_pixel: usize,
) -> usize {
    let gob_address = gob_address(x, y, block_height, image_width_in_gobs, bytes_per_pixel);

    // Multiply by bytes_per_pixel since this function expects byte coordinates.
    // We assume 1 byte per row, so y is left unchanged.
    let gob_offset = gob_offset(x * bytes_per_pixel, y);

    gob_address + gob_offset
}

// TODO: Add a function to calculate surface size by rounding up to blocks (1 x block_height gobs).
// This can be used to calculate the needed dimensions so swizzle deswizzle can simply return the output array.
// For FFI, it will be easier to pass in existing memory of the appropriate size.
// Add surface size calculation to FFI?

/// Calculates the size in bytes for the swizzled data for the given dimensions for the block linear format.
/// The result of [swizzled_surface_size] will always be at least as large as [deswizzled_surface_size].
pub fn swizzled_surface_size(
    width: usize,
    height: usize,
    block_height: usize,
    bytes_per_pixel: usize,
) -> usize {
    let width_in_gobs = width_in_gobs(width, bytes_per_pixel);
    // TODO: Make gob width and gob height constants?
    let height_in_blocks = div_round_up(height, block_height * GOB_HEIGHT);
    width_in_gobs * height_in_blocks * block_height * GOB_SIZE
}

/// Calculates the size in bytes for the deswizzled data for the given dimensions.
/// Compare with [swizzled_surface_size].
pub fn deswizzled_surface_size(width: usize, height: usize, bytes_per_pixel: usize) -> usize {
    width * height * bytes_per_pixel
}

/// Gets the height of each block in GOBs for the specified `height`.
/// For formats that compress multiple pixels into a single tile, divide the height in pixels by the tile height.
/// # Examples
///
/// Non compressed formats can typically just use the height in pixels.
/**
```rust
let height_in_pixels = 512;
assert_eq!(16, nutexb_swizzle::block_height(height_in_pixels));
```
*/
/// BCN formats work in 4x4 tiles instead of pixels, so divide the height by 4 since each tile is 4 pixels high.
/**
```rust
let height_in_pixels = 512;
assert_eq!(16, nutexb_swizzle::block_height(height_in_pixels / 4));
```
*/
pub fn block_height(height: usize) -> usize {
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

fn width_in_gobs(width: usize, bytes_per_pixel: usize) -> usize {
    div_round_up(width * bytes_per_pixel, GOB_WIDTH)
}

// TODO: Avoid panics?

// TODO: Add another option with a specified block height (use an enum)?
// TODO: Add an option to automatically calculate the output size?

/// Swizzles the bytes from `source` into `destination` using the block linear swizzling algorithm.
/// `source` is expected to have at least `width * height * bytes_per_pixel` many bytes.
/// # Panics
/// Panics on out of bounds accesses for `source` or `destination`. This occurs when `source` or `destination` contain too few bytes
/// for the given parameters.
pub fn swizzle_block_linear(
    width: usize,
    height: usize,
    source: &[u8],
    destination: &mut [u8],
    block_height: usize,
    bytes_per_pixel: usize,
) {
    let image_width_in_gobs = width_in_gobs(width, bytes_per_pixel);

    // TODO: Extend this to work with depth as well.
    for y in 0..height {
        for x in 0..width {
            let src = swizzled_address(x, y, block_height, image_width_in_gobs, bytes_per_pixel);
            let dst = (y * width + x) * bytes_per_pixel;

            // Swap the offets for swizzling or deswizzling.
            // TODO: The condition doesn't need to be in the inner loop.
            // TODO: Have an inner function and swap the source/destination arguments in the outer function?
            (&mut destination[src..src + bytes_per_pixel])
                .copy_from_slice(&source[dst..dst + bytes_per_pixel]);
        }
    }
}

// TODO: Add an option to automatically calculate the output size?
// TODO: Return a result instead to make this more robust?
// TODO: Use fuzz testing to test for panics.

/// Deswizzles the bytes from `source` into `destination` using the block linear swizzling algorithm.
/// # Panics
/// Panics on out of bounds accesses for `source` or `destination`.
/// `source` is expected to have at least as many bytes as the result of [swizzled_surface_size].
/// `destination` is expected to have at least as many bytes as the result of [deswizzled_surface_size].
pub fn deswizzle_block_linear(
    width: usize,
    height: usize,
    source: &[u8],
    destination: &mut [u8],
    block_height: usize,
    bytes_per_pixel: usize,
) {
    let image_width_in_gobs = width_in_gobs(width, bytes_per_pixel);

    // TODO: Extend this to work with depth as well.
    for y in 0..height {
        for x in 0..width {
            let src = swizzled_address(x, y, block_height, image_width_in_gobs, bytes_per_pixel);
            let dst = (y * width + x) * bytes_per_pixel;

            (&mut destination[dst..dst + bytes_per_pixel])
                .copy_from_slice(&source[src..src + bytes_per_pixel]);
        }
    }
}

pub mod ffi {
    // TODO: Add another function for correctly calculating the deswizzled size and show a code example.
    // TODO: Show that BCN need width and height divided by 4.
    /// Swizzles the bytes from `source` into `destination` using the block linear swizzling algorithm.
    /// # Safety
    /// `source` and `source_len` should refer to an array with at least as many bytes as the result of [deswizzled_surface_size].
    /// Similarly, `destination` and `destination_len` should refer to an array with at least as many bytes as as the result of [swizzled_surface_size].
    #[no_mangle]
    pub unsafe extern "C" fn swizzle_block_linear(
        width: usize,
        height: usize,
        source: *const u8,
        source_len: usize,
        destination: *mut u8,
        destination_len: usize,
        block_height: usize,
        bytes_per_pixel: usize,
    ) {
        // TODO: Assert that the lengths are correct?
        let source = std::slice::from_raw_parts(source, source_len);
        let destination = std::slice::from_raw_parts_mut(destination, destination_len);

        super::swizzle_block_linear(
            width,
            height,
            source,
            destination,
            block_height,
            bytes_per_pixel,
        )
    }

    /// Deswizzles the bytes from `source` into `destination` using the block linear swizzling algorithm.
    /// # Safety
    /// `source` and `source_len` should refer to an array with at least as many bytes as the result of [swizzled_surface_size].
    /// Similarly, `destination` and `destination_len` should refer to an array with at least as many bytes as as the result of [deswizzled_surface_size].
    #[no_mangle]
    pub unsafe extern "C" fn deswizzle_block_linear(
        width: usize,
        height: usize,
        source: *const u8,
        source_len: usize,
        destination: *mut u8,
        destination_len: usize,
        block_height: usize,
        bytes_per_pixel: usize,
    ) {
        let source = std::slice::from_raw_parts(source, source_len);
        let destination = std::slice::from_raw_parts_mut(destination, destination_len);

        super::deswizzle_block_linear(
            width,
            height,
            source,
            destination,
            block_height,
            bytes_per_pixel,
        )
    }

    /// See [swizzled_surface_size](super::swizzled_surface_size).
    #[no_mangle]
    pub extern "C" fn swizzled_surface_size(
        width: usize,
        height: usize,
        block_height: usize,
        bytes_per_pixel: usize,
    ) -> usize {
        super::swizzled_surface_size(width, height, block_height, bytes_per_pixel)
    }

    /// See [deswizzled_surface_size](super::deswizzled_surface_size).
    #[no_mangle]
    pub extern "C" fn deswizzled_surface_size(
        width: usize,
        height: usize,
        bytes_per_pixel: usize,
    ) -> usize {
        super::deswizzled_surface_size(width, height, bytes_per_pixel)
    }

    /// See [block_height](super::block_height).
    #[no_mangle]
    pub extern "C" fn block_height(height: usize) -> usize {
        super::block_height(height)
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

        deswizzle_block_linear(64 / 4, 64 / 4, input, &mut actual, block_height(64 / 4), 16);

        assert_eq!(expected, &actual[..]);
    }

    #[test]
    fn deswizzle_bc1_128_128() {
        let input = include_bytes!("../../swizzle_data/128_bc1_linear.bin");
        let expected = include_bytes!("../../swizzle_data/128_bc1_linear_deswizzle.bin");
        let mut actual = vec![0u8; 128 * 128 / 16 * 8];

        deswizzle_block_linear(
            128 / 4,
            128 / 4,
            input,
            &mut actual,
            block_height(128 / 4),
            8,
        );

        assert_eq!(expected, &actual[..]);
    }

    #[test]
    fn deswizzle_bc3_128_128() {
        let input = include_bytes!("../../swizzle_data/128_bc3_linear.bin");
        let expected = include_bytes!("../../swizzle_data/128_bc3_linear_deswizzle.bin");
        let mut actual = vec![0u8; 128 * 128];

        // BC3 has the same swizzle patterns as BC7.
        deswizzle_block_linear(
            128 / 4,
            128 / 4,
            input,
            &mut actual,
            block_height(128 / 4),
            16,
        );

        assert_eq!(expected, &actual[..]);
    }

    #[test]
    fn deswizzle_rgba_f32_128_128() {
        let input = include_bytes!("../../swizzle_data/128_rgbaf32_linear.bin");
        let expected = include_bytes!("../../swizzle_data/128_rgbaf32_linear_deswizzle.bin");
        let mut actual = vec![0u8; 128 * 128 * 16];

        // R32G32B32A32_FLOAT has the same swizzle patterns as BC7.
        deswizzle_block_linear(128, 128, input, &mut actual, block_height(128), 16);

        assert_eq!(expected, &actual[..]);
    }

    #[test]
    fn deswizzle_bc7_128_128() {
        let input = include_bytes!("../../swizzle_data/128_bc7_linear.bin");
        let expected = include_bytes!("../../swizzle_data/128_bc7_linear_deswizzle.bin");
        let mut actual = vec![0u8; 128 * 128];

        deswizzle_block_linear(
            128 / 4,
            128 / 4,
            input,
            &mut actual,
            block_height(128 / 4),
            16,
        );

        assert_eq!(expected, &actual[..]);
    }

    #[test]
    fn deswizzle_bc7_256_256() {
        let input = include_bytes!("../../swizzle_data/256_bc7_linear.bin");
        let expected = include_bytes!("../../swizzle_data/256_bc7_linear_deswizzle.bin");
        let mut actual = vec![0u8; 256 * 256];

        deswizzle_block_linear(
            256 / 4,
            256 / 4,
            input,
            &mut actual,
            block_height(256 / 4),
            16,
        );

        assert_eq!(expected, &actual[..]);
    }

    #[test]
    fn deswizzle_bc7_320_320() {
        let input = include_bytes!("../../swizzle_data/320_bc7_linear.bin");
        let expected = include_bytes!("../../swizzle_data/320_bc7_linear_deswizzle.bin");
        let mut actual = vec![0u8; 320 * 320];

        deswizzle_block_linear(
            320 / 4,
            320 / 4,
            input,
            &mut actual,
            block_height(320 / 4),
            16,
        );

        assert_eq!(expected, &actual[..]);
    }

    #[test]
    fn deswizzle_bc7_512_512() {
        let input = include_bytes!("../../swizzle_data/512_bc7_linear.bin");
        let expected = include_bytes!("../../swizzle_data/512_bc7_linear_deswizzle.bin");
        let mut actual = vec![0u8; 512 * 512];

        deswizzle_block_linear(
            512 / 4,
            512 / 4,
            input,
            &mut actual,
            block_height(512 / 4),
            16,
        );

        assert_eq!(expected, &actual[..]);
    }

    #[test]
    fn deswizzle_bc7_1024_1024() {
        let input = include_bytes!("../../swizzle_data/1024_bc7_linear.bin");
        let expected = include_bytes!("../../swizzle_data/1024_bc7_linear_deswizzle.bin");
        let mut actual = vec![0u8; 1024 * 1024];

        deswizzle_block_linear(
            1024 / 4,
            1024 / 4,
            input,
            &mut actual,
            block_height(1024 / 4),
            16,
        );

        assert_eq!(expected, &actual[..]);
    }

    #[test]
    fn width_in_gobs_block16() {
        assert_eq!(20, width_in_gobs(320 / 4, 16));
    }

    #[test]
    fn block_heights() {
        assert_eq!(8, block_height(64));

        // BCN Tiles.
        assert_eq!(16, block_height(768 / 4));
        assert_eq!(16, block_height(384 / 4));
        assert_eq!(16, block_height(384 / 4));
        assert_eq!(8, block_height(320 / 4));
        assert_eq!(4, block_height(80 / 4));
    }

    #[test]
    fn surface_sizes_block4() {
        assert_eq!(
            1048576,
            swizzled_surface_size(512, 512, block_height(512), 4)
        );
    }

    #[test]
    fn surface_sizes_block16() {
        assert_eq!(
            163840,
            swizzled_surface_size(320 / 4, 320 / 4, block_height(320 / 4), 16)
        );
        assert_eq!(
            40960,
            swizzled_surface_size(160 / 4, 160 / 4, block_height(160 / 4), 16)
        );
        assert_eq!(
            1024,
            swizzled_surface_size(32 / 4, 32 / 4, block_height(32 / 4), 16)
        );
    }
}
