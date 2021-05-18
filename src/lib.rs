use binread::prelude::*;
use std::{
    io::{Cursor, Write},
    path::Path,
};

mod nutexb;
mod swizzle;

pub enum ImageFormat {
    Rgba,
    Bc1,
    Bc3,
    Bc7
}

pub fn swizzle_bc3_bc7<P: AsRef<Path>>(input: P, output: P, width: usize, height: usize) {
    let input_data = read_blocks::<_, u128>(input);

    // Swizzling is currently being done on blocks or tiles rather than by byte addresses.
    let (x_mask, y_mask) = swizzle::calculate_swizzle_pattern(width as u32, height as u32);

    let width_in_blocks = width / 4;
    let height_in_blocks = height / 4;

    let mut output_data = vec![0u128; width_in_blocks * height_in_blocks];
    swizzle::swizzle_experimental(x_mask as i32, y_mask as i32, width_in_blocks, height_in_blocks, &input_data, &mut output_data[..], false);

    let mut writer = std::fs::File::create(output).unwrap();
    for value in output_data {
        writer.write_all(&value.to_le_bytes()).unwrap();
    }
}

// TODO: Avoid repetitive code.
pub fn deswizzle_bc3_bc7<P: AsRef<Path>>(input: P, output: P, width: usize, height: usize) {
    let input_data = read_blocks::<_, u128>(input);

    // Swizzling is currently being done on blocks or tiles rather than by byte addresses.
    let (x_mask, y_mask) = swizzle::calculate_swizzle_pattern(width as u32, height as u32);

    let width_in_blocks = width / 4;
    let height_in_blocks = height / 4;

    let mut output_data = vec![0u128; width_in_blocks * height_in_blocks];
    swizzle::swizzle_experimental(x_mask as i32, y_mask as i32, width_in_blocks, height_in_blocks, &input_data, &mut output_data[..], true);

    let mut writer = std::fs::File::create(output).unwrap();
    for value in output_data {
        writer.write_all(&value.to_le_bytes()).unwrap();
    }
}

pub fn try_get_image_format(format: &str) -> std::result::Result<ImageFormat, &str> {
    match format {
        "rgba" => Ok(ImageFormat::Rgba),
        "bc1" => Ok(ImageFormat::Bc1),
        "bc3" => Ok(ImageFormat::Bc3),
        "bc7" => Ok(ImageFormat::Bc7),
        _ => Err("Unsupported format"),
    }
}

fn read_blocks<P: AsRef<Path>, T: BinRead>(path: P) -> Vec<T> {
    let mut raw = Cursor::new(std::fs::read(path).unwrap());

    let mut blocks = Vec::new();
    while let Ok(block) = raw.read_le::<T>() {
        blocks.push(block);
    }
    blocks
}

fn create_deswizzle_lut<T: PartialEq>(
    linear_blocks: &[T],
    deswizzled_blocks: &[T],
) -> Vec<i64> {
    // For each deswizzled output block index, find the corresponding input block index.
    // This is O(n^2) where n is the number of blocks since we don't decode the block data to get the index.
    let mut output = Vec::new();
    for block in deswizzled_blocks.iter() {
        match linear_blocks.iter().position(|b| b == block) {
            Some(value) => output.push(value as i64),
            None => {
                output.push(-1);
            }
        }
    }

    output
}

fn deswizzle_blocks<T: Default + Copy + Clone>(
    swizzled_blocks: &[T],
    deswizzle_lut: &[i64],
) -> Vec<T> {
    let mut deswizzled_blocks = vec![T::default(); swizzled_blocks.len()];
    for (i, linear) in deswizzle_lut.iter().enumerate() {
        if *linear >= 0 {
            deswizzled_blocks[i] = swizzled_blocks[*linear as usize];
        }
    }

    deswizzled_blocks
}

fn swizzle_blocks<T: Default + Copy + Clone>(
    linear_blocks: &[T],
    deswizzle_lut: &[i64],
) -> Vec<T> {
    let mut swizzled_blocks = vec![T::default(); linear_blocks.len()];
    for (i, linear) in deswizzle_lut.iter().enumerate() {
        if *linear >= 0 {
            swizzled_blocks[*linear as usize] = linear_blocks[i];
        }
    }

    swizzled_blocks
}

pub fn write_rgba_lut<W: Write>(writer: &mut W, pixel_count: usize) {
    for i in 0..pixel_count as u32 {
        // Use the linear address to create unique pixel values.
        writer.write_all(&i.to_le_bytes()).unwrap();
    }
}

pub fn write_bc7_lut<W: Write>(writer: &mut W, block_count: usize) {
    for i in 0..block_count as u64 {
        // Create 128 bits of unique BC7 data.
        // We just need unique blocks rather than unique pixel colors.
        writer.write_all(&0u32.to_le_bytes()).unwrap();
        writer.write_all(&i.to_le_bytes()).unwrap();
        writer.write_all(&2u32.to_le_bytes()).unwrap();
    }
}

pub fn write_bc3_lut<W: Write>(writer: &mut W, block_count: usize) {
    for i in 0..block_count as u64 {
        // Create 128 bits of unique BC3 data.
        // We just need unique blocks rather than unique pixel colors.
        writer.write_all(&65535u64.to_le_bytes()).unwrap();
        writer.write_all(&i.to_le_bytes()).unwrap();
    }
}

pub fn write_bc1_lut<W: Write>(writer: &mut W, block_count: usize) {
    for i in 0..block_count as u32 {
        // Create 64 bits of unique BC1 data.
        // We just need unique blocks rather than unique pixel colors.
        writer.write_all(&0u32.to_le_bytes()).unwrap();
        writer.write_all(&i.to_le_bytes()).unwrap();
    }
}

fn print_swizzle_patterns(deswizzle_lut: &[i64], width: usize, height: usize) {
    println!("width: {:?}, height: {:?}", width, height);
    let tile_size = 4;
    let x_pattern_index = (width / tile_size) - 1;
    let y_pattern_index = (width / tile_size) * (height / tile_size - 1);

    // Left pad to 16 to align the output for dimensions up to (65536, 65536).
    println!(
        "x: {:016b}", deswizzle_lut[x_pattern_index]
    );
    println!(
        "y: {:016b}", deswizzle_lut[y_pattern_index]
    );
    // "Invert" the deswizzle LUT to create the swizzle LUT.
    // println!(
    //     "x: {:016b}",
    //     deswizzle_lut
    //         .iter()
    //         .position(|i| *i as usize == x_pattern_index)
    //         .unwrap()
    // );
    // println!(
    //     "y: {:016b}",
    //     deswizzle_lut
    //         .iter()
    //         .position(|i| *i as usize == y_pattern_index)
    //         .unwrap()
    // );
}

pub fn calculate_swizzle_patterns<
    T: BinRead + PartialEq + std::fmt::Debug + Default + Copy + std::fmt::LowerHex,
    P: AsRef<Path>,
>(
    swizzled_file: P,
    deswizzled_file: P,
    width: usize,
    height: usize,
    deswizzled_block_count: usize
) {
    let linear_blocks = read_blocks::<_, T>(swizzled_file);
    let deswizzled_blocks = read_blocks::<_, T>(deswizzled_file);

    let lut = create_deswizzle_lut(&linear_blocks, &deswizzled_blocks);
    print_swizzle_patterns(&lut, width, height);

    // TODO: This probably should have less verbose output on failure.
    assert_eq!(deswizzle_blocks(&linear_blocks, &lut)[..deswizzled_block_count], deswizzled_blocks[..deswizzled_block_count]);
    assert_eq!(swizzle_blocks(&deswizzled_blocks, &lut)[..deswizzled_block_count], linear_blocks[..deswizzled_block_count]);
}

pub fn create_nutexb(
    writer: &mut std::fs::File,
    width: usize,
    height: usize,
    name: &str,
    format: &ImageFormat,
    block_count: usize
) {
    let nutexb_format = match format {
        ImageFormat::Rgba => 0,
        ImageFormat::Bc1 => 0x90,
        ImageFormat::Bc3 => 0xa0,
        ImageFormat::Bc7 => 0xe0,
    };

    let mut buffer = Cursor::new(Vec::new());
    match format {
        ImageFormat::Rgba => write_rgba_lut(&mut buffer, block_count),
        ImageFormat::Bc1 => write_bc1_lut(&mut buffer, block_count),
        ImageFormat::Bc3 => write_bc3_lut(&mut buffer, block_count),
        ImageFormat::Bc7 => write_bc7_lut(&mut buffer, block_count),
    }

    nutexb::write_nutexb_from_data(
        writer,
        buffer.get_ref(),
        width as u32,
        height as u32,
        name,
        nutexb_format,
    )
    .unwrap();
}
