use binread::prelude::*;
use std::{
    io::{Cursor, Write},
    path::Path,
};

use crate::swizzle::{swizzle_x_16, swizzle_x_8, swizzle_y_16, swizzle_y_8};

mod nutexb;
mod swizzle;

pub enum ImageFormat {
    Rgba8,
    RgbaF32,
    Bc1,
    Bc3,
    Bc7,
}

pub fn swizzle_data(
    input_data: &[u8],
    width: usize,
    height: usize,
    format: &ImageFormat,
) -> Vec<u8> {
    let width_in_blocks = width / 4;
    let height_in_blocks = height / 4;

    let tile_size = get_tile_size(format);

    let mut output_data = vec![0u8; width_in_blocks * height_in_blocks * tile_size];
    // TODO: Support other formats.
    match format {
        ImageFormat::Rgba8 => {}
        ImageFormat::Bc1 => swizzle::swizzle_experimental(
            swizzle_x_8,
            swizzle_y_8,
            width_in_blocks,
            height_in_blocks,
            &input_data,
            &mut output_data[..],
            false,
            8,
        ),
        ImageFormat::Bc3 | ImageFormat::Bc7 => swizzle::swizzle_experimental(
            swizzle_x_16,
            swizzle_y_16,
            width_in_blocks,
            height_in_blocks,
            &input_data,
            &mut output_data[..],
            false,
            16,
        ),
        ImageFormat::RgbaF32 => swizzle::swizzle_experimental(
            swizzle_x_16,
            swizzle_y_16,
            width,
            height,
            &input_data,
            &mut output_data[..],
            false,
            16,
        )
    }

    output_data
}

pub fn swizzle<P: AsRef<Path>>(
    input: P,
    output: P,
    width: usize,
    height: usize,
    format: &ImageFormat,
) {
    let input_data = std::fs::read(input).unwrap();
    let output_data = swizzle_data(&input_data, width, height, format);

    let mut writer = std::fs::File::create(output).unwrap();
    for value in output_data {
        writer.write_all(&value.to_le_bytes()).unwrap();
    }
}

pub fn deswizzle_data(
    input_data: &[u8],
    width: usize,
    height: usize,
    format: &ImageFormat,
) -> Vec<u8> {
    // TODO: This isn't correct for RGBA.
    let width_in_blocks = width / 4;
    let height_in_blocks = height / 4;

    let tile_size = get_tile_size(format);

    let mut output_data = vec![0u8; width_in_blocks * height_in_blocks * tile_size];
    // TODO: Support other formats.
    match format {
        // TODO: This can just be based on block size rather than image format.
        ImageFormat::Rgba8 => {}
        ImageFormat::Bc1 => swizzle::swizzle_experimental(
            swizzle_x_8,
            swizzle_y_8,
            width_in_blocks,
            height_in_blocks,
            &input_data,
            &mut output_data[..],
            true,
            8,
        ),
        ImageFormat::Bc3 | ImageFormat::Bc7 => swizzle::swizzle_experimental(
            swizzle_x_16,
            swizzle_y_16,
            width_in_blocks,
            height_in_blocks,
            &input_data,
            &mut output_data[..],
            true,
            16,
        ),
        ImageFormat::RgbaF32 => swizzle::swizzle_experimental(
            swizzle_x_16,
            swizzle_y_16,
            width,
            height,
            &input_data,
            &mut output_data[..],
            true,
            16,
        )
    }

    output_data
}

// TODO: Avoid repetitive code.
pub fn deswizzle<P: AsRef<Path>>(
    input: P,
    output: P,
    width: usize,
    height: usize,
    format: &ImageFormat,
) {
    let input_data = std::fs::read(input).unwrap();
    let output_data = deswizzle_data(&input_data, width, height, format);

    let mut writer = std::fs::File::create(output).unwrap();
    for value in output_data {
        writer.write_all(&value.to_le_bytes()).unwrap();
    }
}

pub fn try_get_image_format(format: &str) -> std::result::Result<ImageFormat, &str> {
    match format {
        "rgba8" => Ok(ImageFormat::Rgba8),
        "rgbaf32" => Ok(ImageFormat::RgbaF32),
        "bc1" => Ok(ImageFormat::Bc1),
        "bc3" => Ok(ImageFormat::Bc3),
        "bc7" => Ok(ImageFormat::Bc7),
        _ => Err("Unsupported format"),
    }
}

fn get_tile_size(format: &ImageFormat) -> usize {
    match format {
        ImageFormat::Rgba8 => 4,
        ImageFormat::RgbaF32 => 16,
        ImageFormat::Bc1 => 8,
        ImageFormat::Bc3 | ImageFormat::Bc7 => 16,
    }
}

fn read_vec<T: BinRead, R: BinReaderExt>(reader: &mut R) -> Vec<T> {
    let mut result = Vec::new();
    while let Ok(block) = reader.read_le::<T>() {
        result.push(block);
    }
    result
}

fn read_blocks<P: AsRef<Path>, T: BinRead>(path: P) -> Vec<T> {
    let mut raw = Cursor::new(std::fs::read(path).unwrap());
    read_vec(&mut raw)
}

fn read_mipmaps_dds<P: AsRef<Path>, T: BinRead>(path: P) -> Vec<Vec<T>> {
    let mut reader = std::fs::File::open(path).unwrap();
    let dds = ddsfile::Dds::read(&mut reader).unwrap();

    // Each mip level is 4x smaller than the previous level.
    let mut mip_offset = 0;
    let mut mip_size = dds.get_main_texture_size().unwrap() as usize;
    let min_mipmap_size = dds.get_min_mipmap_size_in_bytes() as usize;

    let mut mip_data = Vec::new();
    for _ in 0..dds.get_num_mipmap_levels() {
        let mut reader = Cursor::new(&dds.data[mip_offset..mip_offset + mip_size]);
        let blocks = read_vec(&mut reader);
        mip_data.push(blocks);

        // Some compressed formats have a minimum size.
        mip_offset += std::cmp::max(mip_size, min_mipmap_size);
        mip_size /= 4;
    }

    mip_data
}

fn create_deswizzle_luts<T: PartialEq>(
    linear_mipmaps: &[Vec<T>],
    deswizzled_mipmaps: &[Vec<T>],
) -> Vec<Vec<i64>> {
    let mut luts = Vec::new();

    for (linear_mip, deswizzled_mip) in deswizzled_mipmaps.iter().zip(linear_mipmaps) {
        let mip_lut = create_mip_deswizzle_lut(linear_mip, deswizzled_mip);
        luts.push(mip_lut);
    }

    luts
}

fn create_mip_deswizzle_lut<T: PartialEq>(linear: &[T], deswizzled: &[T]) -> Vec<i64> {
    // For each deswizzled output block index, find the corresponding input block index.
    // This is O(n^2) where n is the number of blocks since we don't decode the block data to get the index.
    let mut mip_lut = Vec::new();
    for block in deswizzled {
        match linear.iter().position(|b| b == block) {
            Some(value) => mip_lut.push(value as i64),
            None => {
                mip_lut.push(-1);
            }
        }
    }

    mip_lut
}

// TODO: Return result?
pub fn write_rgba_lut<W: Write>(writer: &mut W, pixel_count: usize) {
    for i in 0..pixel_count as u32 {
        // Use the linear address to create unique pixel values.
        writer.write_all(&i.to_le_bytes()).unwrap();
    }
}

pub fn write_rgba_f32_lut<W: Write>(writer: &mut W, pixel_count: usize) {
    for i in 0..pixel_count {
        // Use the linear address to create unique pixel values.
        // TODO: This only works up to 16777216.
        // TODO: Flip sign bit for larger values?
        writer.write_all(&(i as f32).to_le_bytes()).unwrap();
        writer.write_all(&0f32.to_le_bytes()).unwrap();
        writer.write_all(&0f32.to_le_bytes()).unwrap();
        writer.write_all(&0f32.to_le_bytes()).unwrap();
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

fn print_swizzle_patterns(
    deswizzle_lut: &[i64],
    width: usize,
    height: usize,
    tile_dimension: usize,
) {
    if width == 0 || height == 0 || deswizzle_lut.is_empty() {
        return;
    }

    println!("width: {:?}, height: {:?}", width, height);
    let width_in_tiles = width / tile_dimension;
    let height_in_tiles = height / tile_dimension;

    let x_pattern_index = if width_in_tiles > 1 {
        width_in_tiles - 1
    } else {
        0
    };
    let y_pattern_index = if height_in_tiles > 1 {
        width_in_tiles * (height_in_tiles - 1)
    } else {
        0
    };

    println!("x: {:032b}", deswizzle_lut[x_pattern_index]);
    println!("y: {:032b}", deswizzle_lut[y_pattern_index]);
}

pub fn guess_swizzle_patterns<T: BinRead + PartialEq + Default + Copy, P: AsRef<Path>>(
    swizzled_file: P,
    deswizzled_file: P,
    width: usize,
    height: usize,
    format: &ImageFormat,
) {
    let swizzled_mipmaps = match std::path::Path::new(swizzled_file.as_ref())
        .extension()
        .unwrap()
        .to_str()
        .unwrap()
    {
        "dds" => read_mipmaps_dds(&swizzled_file),
        _ => vec![read_blocks::<_, T>(&swizzled_file)],
    };

    let deswizzled_mipmaps = match std::path::Path::new(deswizzled_file.as_ref())
        .extension()
        .unwrap()
        .to_str()
        .unwrap()
    {
        "dds" => read_mipmaps_dds(&deswizzled_file),
        _ => vec![read_blocks::<_, T>(&deswizzled_file)],
    };

    if swizzled_mipmaps.len() == 1 && deswizzled_mipmaps.len() > 1 {
        // Assume the input blocks cover all mip levels.
        // This allows for calculating mip offsets and sizes.
        let mut mip_width = width;
        let mut mip_height = height;
        for mip in deswizzled_mipmaps {
            // TODO: Is this necessary for all formats?
            if mip_width < 4 || mip_height < 4 {
                break;
            }

            // Calculate the start and end of the mipmap based on block indices.
            let mip_lut = create_mip_deswizzle_lut(&swizzled_mipmaps[0], &mip);
            let start_index = mip_lut.iter().min().unwrap();
            let end_index = mip_lut.iter().max().unwrap();
            println!("Start Index: {:?}", start_index);
            println!("End Index: {:?}", end_index);

            // For the swizzle patterns, assume the swizzling starts from the mipmap offset.
            let mut mip_lut = create_mip_deswizzle_lut(&swizzled_mipmaps[0], &mip);
            for val in mip_lut.iter_mut() {
                *val -= start_index;
            }

            match format {
                ImageFormat::Rgba8 => print_swizzle_patterns(&mip_lut, mip_width, mip_height, 1),
                _ => print_swizzle_patterns(&mip_lut, mip_width, mip_height, 4),
            }
            println!();

            mip_width /= 2;
            mip_height /= 2;
        }
    } else {
        // Compare both mipmaps.
        let lut = create_deswizzle_luts(&swizzled_mipmaps, &deswizzled_mipmaps);

        let mut mip_width = width;
        let mut mip_height = height;
        for mip_lut in lut {
            // TODO: Is this necessary for all formats?
            if mip_width < 4 || mip_height < 4 {
                break;
            }

            match format {
                ImageFormat::Rgba8 => print_swizzle_patterns(&mip_lut, mip_width, mip_height, 1),
                _ => print_swizzle_patterns(&mip_lut, mip_width, mip_height, 4),
            }
            mip_width /= 2;
            mip_height /= 2;
        }
    }
}

pub fn create_nutexb<W: Write>(
    writer: &mut W,
    width: usize,
    height: usize,
    name: &str,
    format: &ImageFormat,
    block_count: usize,
) {
    let nutexb_format = match format {
        ImageFormat::Rgba8 => 0,
        ImageFormat::Bc1 => 128,
        ImageFormat::Bc3 => 160,
        ImageFormat::Bc7 => 224,
        ImageFormat::RgbaF32 => 52,
    };

    let mut buffer = Cursor::new(Vec::new());
    match format {
        ImageFormat::Rgba8 => write_rgba_lut(&mut buffer, block_count),
        ImageFormat::Bc1 => write_bc1_lut(&mut buffer, block_count),
        ImageFormat::Bc3 => write_bc3_lut(&mut buffer, block_count),
        ImageFormat::Bc7 => write_bc7_lut(&mut buffer, block_count),
        ImageFormat::RgbaF32 => write_rgba_f32_lut(&mut buffer, block_count),
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
