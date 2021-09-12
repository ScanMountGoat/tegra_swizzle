#![no_main]
use libfuzzer_sys::fuzz_target;

extern crate arbitrary;
use arbitrary::{Arbitrary, Result, Unstructured};

#[derive(Debug)]
struct Input {
    width: usize,
    height: usize,
    depth: usize,
    block_height: nutexb_swizzle::BlockHeight,
    bytes_per_pixel: usize,
}

impl<'a> Arbitrary<'a> for Input {
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
        Ok(Input {
            width: u.int_in_range(0..=8096)?,
            height: u.int_in_range(0..=8096)?,
            depth: 1,
            block_height: u.arbitrary()?,
            bytes_per_pixel: u.int_in_range(0..=32)?,
        })
    }
}

fuzz_target!(|input: Input| {
    // fuzzed code goes here
    let swizzled = vec![
        0u8;
        nutexb_swizzle::swizzled_surface_size(
            input.width,
            input.height,
            input.depth,
            input.block_height,
            input.bytes_per_pixel
        )
    ];

    nutexb_swizzle::deswizzle_block_linear(
        input.width,
        input.height,
        input.depth,
        &swizzled,
        input.block_height,
        input.bytes_per_pixel,
    );
});
