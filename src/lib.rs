use binread::prelude::*;
use std::{
    io::{Cursor, Write},
    path::Path,
};

mod nutexb;

fn swizzle_experimental<T: Copy>(
    x_mask: i32,
    y_mask: i32,
    width: usize,
    height: usize,
    source: &[T],
    destination: &mut [T],
    deswizzle: bool,
    element_count_per_copy: usize,
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
                (&mut destination[dst..dst + element_count_per_copy])
                    .copy_from_slice(&source[src..src + element_count_per_copy]);
            } else {
                (&mut destination[src..src + element_count_per_copy])
                    .copy_from_slice(&source[dst..dst + element_count_per_copy]);
            }

            // Use the 2's complement identity (offset + !mask + 1 == offset - mask).
            offset_x = (offset_x - x_mask) & x_mask;
            dst += element_count_per_copy;
        }
        offset_y = (offset_y - y_mask) & y_mask;
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

// TODO: Proper error handling.
fn create_swizzled_to_linear_lut<T: PartialEq + std::fmt::Debug + std::fmt::LowerHex>(
    linear_blocks: &[T],
    deswizzled_blocks: &[T],
) -> Vec<i64> {
    // TODO: Proper error handling.

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

    let lut = create_swizzled_to_linear_lut(&linear_blocks, &deswizzled_blocks);
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
    format: &str,
    block_count: usize
) {
    let nutexb_format = match format {
        "rgba" => 0,
        "bc1" => 0x90,
        "bc3" => 0xa0,
        "bc7" => 0xe0,
        _ => unreachable!(),
    };

    let mut buffer = Cursor::new(Vec::new());
    // TODO: RGBA
    match format {
        "bc1" => write_bc1_lut(&mut buffer, block_count),
        "bc3" => write_bc3_lut(&mut buffer, block_count),
        "bc7" => write_bc7_lut(&mut buffer, block_count),
        _ => unreachable!(),
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
