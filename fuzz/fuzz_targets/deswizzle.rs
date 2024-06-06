#![no_main]
use libfuzzer_sys::fuzz_target;

extern crate arbitrary;
use arbitrary::{Arbitrary, Result, Unstructured};

#[derive(Debug)]
struct Input {
    width: u32,
    height: u32,
    depth: u32,
    block_height: tegra_swizzle::BlockHeight,
    bytes_per_pixel: u32,
    input_size: usize,
}

impl<'a> Arbitrary<'a> for Input {
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
        Ok(Input {
            width: u.int_in_range(0..=4096)?,
            height: u.int_in_range(0..=4096)?,
            depth: 1,
            block_height: u.arbitrary()?,
            bytes_per_pixel: u.int_in_range(0..=32)?,
            input_size: u.int_in_range(0..=16777216)?,
        })
    }
}

fuzz_target!(|input: Input| {
    let swizzled = vec![0u8; input.input_size];

    // This should never panic even if the input size is incorrect.
    tegra_swizzle::swizzle::deswizzle_block_linear(
        input.width,
        input.height,
        input.depth,
        &swizzled,
        input.block_height,
        input.bytes_per_pixel,
    );
});
