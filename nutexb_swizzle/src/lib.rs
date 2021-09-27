use ahash::AHashMap;
use binread::prelude::*;
use binwrite::BinWrite;
use formats::ImageFormat;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::{
    fs::File,
    io::{BufWriter, Cursor, Write},
    path::Path,
};

pub mod formats;
mod nutexb;

/// The necessary trait bounds for types that can be used for swizzle calculation functions.
/// The [u32], [u64], and [u128] types implement the necessary traits and can be used to represent block sizes of 4, 8, and 16 bytes, respectively.
pub trait LookupBlock:
    BinRead + Eq + PartialEq + Default + Copy + Send + Sync + std::hash::Hash
{
}
impl<T: BinRead + Eq + PartialEq + Default + Copy + Send + Sync + std::hash::Hash> LookupBlock
    for T
{
}

fn block_height(height: usize) -> tegra_swizzle::BlockHeight {
    let block_height = height / 8;

    // TODO: Is it correct to find the closest power of two?
    // TODO: This is only valid for nutexb, so it likely shouldn't be part of this API.
    match block_height {
        0..=1 => tegra_swizzle::BlockHeight::One,
        2 => tegra_swizzle::BlockHeight::Two,
        3..=4 => tegra_swizzle::BlockHeight::Four,
        5..=8 => tegra_swizzle::BlockHeight::Eight, // TODO: This doesn't work for 320x320 BC7 mip0?
        // TODO: The TRM mentions 32 also works?
        _ => tegra_swizzle::BlockHeight::Sixteen,
    }
}

pub fn swizzle_data(
    input_data: &[u8],
    width: usize,
    height: usize,
    format: &ImageFormat,
) -> Vec<u8> {
    let width_in_tiles = width / format.tile_dimension();
    let height_in_tiles = height / format.tile_dimension();

    let tile_size = format.tile_size_in_bytes();

    let block_height = block_height(height_in_tiles);

    let output_data = tegra_swizzle::swizzle_block_linear(
        width_in_tiles,
        height_in_tiles,
        1,
        input_data,
        block_height,
        tile_size,
    )
    .unwrap();

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
    let width_in_tiles = width / format.tile_dimension();
    let height_in_tiles = height / format.tile_dimension();

    let tile_size = format.tile_size_in_bytes();

    let block_height = block_height(height_in_tiles);

    let output_data = tegra_swizzle::deswizzle_block_linear(
        width_in_tiles,
        height_in_tiles,
        1,
        input_data,
        block_height,
        tile_size,
    )
    .unwrap();

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

fn create_deswizzle_luts<T: LookupBlock>(
    linear_mipmaps: &[Vec<T>],
    deswizzled_mipmaps: &[Vec<T>],
) -> Vec<Vec<i64>> {
    let mut luts = Vec::new();

    for (linear_mip, deswizzled_mip) in deswizzled_mipmaps.iter().zip(linear_mipmaps) {
        let mip_lut = create_swizzle_lut(linear_mip, deswizzled_mip);
        luts.push(mip_lut);
    }

    luts
}

fn create_swizzle_lut<T: LookupBlock>(swizzled: &[T], deswizzled: &[T]) -> Vec<i64> {
    // For each deswizzled output block index, find the corresponding input block index.
    // The lookup table allows for iterating the input lists only once for an O(n) running time.
    let mut swizzled_index_by_block = AHashMap::with_capacity(swizzled.len());
    for (i, value) in swizzled.iter().enumerate() {
        swizzled_index_by_block.insert(value, i);
    }

    // The resulting LUT finds the index after swizzling for a given input index.
    deswizzled
        .par_iter()
        .map(|block| {
            swizzled_index_by_block
                .get(block)
                .map(|i| *i as i64)
                .unwrap_or(-1)
        })
        .collect()
}

// TODO: Return result?
pub fn write_rgba_lut<W: Write>(writer: &mut W, pixel_count: usize) {
    for i in 0..pixel_count as u32 {
        // Use the linear address to create unique pixel values.
        writer.write_all(&(i / 128).to_le_bytes()).unwrap();
    }
}

pub fn write_rgba_f32_lut<W: Write>(writer: &mut W, pixel_count: usize) {
    for i in 0..pixel_count {
        // Use the linear address to create unique pixel values.
        // Writing the index directly would result in values being clipped to 0f32.
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

fn get_swizzle_patterns_output(
    deswizzle_lut: &[i64],
    width: usize,
    height: usize,
    tile_dimension: usize,
) -> String {
    if width == 0 || height == 0 || deswizzle_lut.is_empty() {
        return String::new();
    }

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

    return format!(
        "width: {:?}, height: {:?}\nx: {:032b}\ny: {:032b}",
        width, height, deswizzle_lut[x_pattern_index], deswizzle_lut[y_pattern_index]
    );
}

fn mipmap_range(lut: &[i64]) -> (i64, i64) {
    (*lut.iter().min().unwrap(), *lut.iter().max().unwrap())
}

pub fn write_lut_csv<P: AsRef<Path>>(
    swizzled_file: P,
    deswizzled_file: P,
    output_csv: P,
    format: &ImageFormat,
    normalize_indices: bool,
) {
    // TODO: Tile size should be an enum.
    // TODO: Associate block types with each variant?
    match format.tile_size_in_bytes() {
        4 => write_lut_csv_inner::<u32, _>(
            swizzled_file,
            deswizzled_file,
            output_csv,
            normalize_indices,
        ),
        8 => write_lut_csv_inner::<u64, _>(
            swizzled_file,
            deswizzled_file,
            output_csv,
            normalize_indices,
        ),
        16 => write_lut_csv_inner::<u128, _>(
            swizzled_file,
            deswizzled_file,
            output_csv,
            normalize_indices,
        ),
        _ => (),
    }
}

// TODO: Handle errors.
fn write_lut_csv_inner<T: LookupBlock, P: AsRef<Path>>(
    swizzled_file: P,
    deswizzled_file: P,
    output_csv: P,
    normalize_indices: bool,
) {
    let swizzled_data = read_blocks::<_, T>(&swizzled_file);
    let deswizzled_data = read_blocks::<_, T>(&deswizzled_file);
    let mut swizzle_lut = create_swizzle_lut(&swizzled_data, &deswizzled_data);

    // Ensure indices start from 0.
    if normalize_indices {
        let (start_index, _) = mipmap_range(&swizzle_lut);
        for val in swizzle_lut.iter_mut() {
            *val -= start_index;
        }
    }

    let mut writer = csv::Writer::from_path(output_csv).unwrap();
    writer.serialize(("input_index", "swizzled_index")).unwrap();
    for (input, output) in swizzle_lut.iter().enumerate() {
        writer.serialize((input, output)).unwrap();
    }
}

pub fn print_swizzle_patterns<T: LookupBlock, P: AsRef<Path>>(
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

    // TODO: There is a lot of repetition for these two conditions.
    if swizzled_mipmaps.len() == 1 && deswizzled_mipmaps.len() > 1 {
        // Associate each mipmap with its mip level to avoid having to use enumerate with rayon.
        let deswizzled_mipmaps: Vec<_> = deswizzled_mipmaps.iter().enumerate().collect();

        // The mipmaps can now be computed independently.
        // Collect will ensure the outputs are still displayed in the expected order.
        let mip_outputs: Vec<_> = deswizzled_mipmaps
            .par_iter()
            .map(|(i, mip)| {
                // TODO: Is this necessary for all formats?
                let mip_width = width / (2usize.pow(*i as u32));
                let mip_height = height / (2usize.pow(*i as u32));
                if mip_width < 4 || mip_height < 4 {
                    return String::new();
                }

                // Assume the input blocks cover all mip levels.
                // This allows for calculating mip offsets and sizes based on the range of block indices.
                let mut mip_lut = create_swizzle_lut(&swizzled_mipmaps[0], mip);
                let (start_index, end_index) = mipmap_range(&mip_lut);

                // For the swizzle patterns, assume the swizzling starts from the mipmap offset.
                for val in mip_lut.iter_mut() {
                    *val -= start_index;
                }

                let tile_dimension = format.tile_dimension();
                let swizzle_output =
                    get_swizzle_patterns_output(&mip_lut, mip_width, mip_height, tile_dimension);

                format!(
                    "Start Index: {:?}\nEnd Index: {:?}\n{}\n",
                    start_index, end_index, swizzle_output
                )
            })
            .collect();

        for output in mip_outputs {
            println!("{}", output);
        }
    } else {
        // Compare both mipmaps.
        let mip_luts = create_deswizzle_luts(&swizzled_mipmaps, &deswizzled_mipmaps);
        let mip_luts: Vec<_> = mip_luts.iter().enumerate().collect();
        // TODO: This can also be done in parallel.
        let mip_outputs: Vec<_> = mip_luts
            .iter()
            .map(|(i, mip_lut)| {
                // TODO: Is this necessary for all formats?
                let mip_width = width / (2usize.pow(*i as u32));
                let mip_height = height / (2usize.pow(*i as u32));
                if mip_width < 4 || mip_height < 4 {
                    return String::new();
                }

                let tile_dimension = format.tile_dimension();

                get_swizzle_patterns_output(mip_lut, mip_width, mip_height, tile_dimension)
            })
            .collect();

        for output in mip_outputs {
            println!("{}", output);
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

pub fn extract_mipmaps(input: &str, output: &str, format: &ImageFormat) {
    // TODO: Support nutexb as well.
    // TODO: Is there a way to return a type?
    match format {
        &ImageFormat::Rgba8 => extract_mipmaps_innner::<u32>(input, output),
        ImageFormat::RgbaF32 => extract_mipmaps_innner::<u128>(input, output),
        ImageFormat::Bc1 => extract_mipmaps_innner::<u64>(input, output),
        ImageFormat::Bc3 => extract_mipmaps_innner::<u128>(input, output),
        ImageFormat::Bc7 => extract_mipmaps_innner::<u128>(input, output),
    }
}

fn extract_mipmaps_innner<T: BinRead + BinWrite>(input: &str, output: &str) {
    let mipmaps = read_mipmaps_dds::<_, T>(input);
    for (i, mip) in mipmaps.into_iter().enumerate() {
        let output_path = format!("{}_{}.bin", output, i);

        // TODO: This will write with native endianness but the input is assumed to be little endian.
        let mut file = BufWriter::new(File::create(output_path).unwrap());
        mip.write(&mut file).unwrap();
    }
}
