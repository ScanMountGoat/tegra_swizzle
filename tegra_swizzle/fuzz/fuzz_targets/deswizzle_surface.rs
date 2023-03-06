#![no_main]
use libfuzzer_sys::fuzz_target;

extern crate arbitrary;
use arbitrary::{Arbitrary, Result, Unstructured};
use std::num::NonZeroUsize;

#[derive(Debug)]
struct Input {
    width: usize,
    height: usize,
    depth: usize,
    block_width: NonZeroUsize,
    block_height: NonZeroUsize,
    block_height_mip0: tegra_swizzle::BlockHeight,
    bytes_per_pixel: usize,
    input_size: usize,
    layer_count: usize,
    mipmap_count: usize,
}

impl<'a> Arbitrary<'a> for Input {
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
        Ok(Input {
            width: u.int_in_range(0..=256)?,
            height: u.int_in_range(0..=256)?,
            depth: u.int_in_range(0..=256)?,
            block_width: NonZeroUsize::new(u.int_in_range(1..=16)?).unwrap(),
            block_height: NonZeroUsize::new(u.int_in_range(1..=16)?).unwrap(),
            block_height_mip0: u.arbitrary()?,
            bytes_per_pixel: u.int_in_range(0..=32)?,
            input_size: u.int_in_range(0..=16777216)?,
            layer_count: u.int_in_range(0..=12)?,
            mipmap_count: u.int_in_range(0..=32)?,
        })
    }
}

fuzz_target!(|input: Input| {
    let swizzled = vec![0u8; input.input_size];

    // This should never panic even if the input size is incorrect.
    tegra_swizzle::surface::deswizzle_surface(
        input.width,
        input.height,
        input.depth,
        &swizzled,
        tegra_swizzle::surface::BlockDim {
            width: input.block_width,
            height: input.block_height,
            depth: NonZeroUsize::new(1).unwrap(),
        },
        Some(input.block_height_mip0),
        input.bytes_per_pixel,
        input.layer_count,
        input.mipmap_count,
    );
});
