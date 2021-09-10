#![no_main]
use libfuzzer_sys::fuzz_target;

extern crate arbitrary;
use arbitrary::{Arbitrary, Result, Unstructured};

#[derive(Debug)]
struct Input {
    width: usize,
    height: usize,
    block_height: usize,
    bytes_per_pixel: usize,
}

impl<'a> Arbitrary<'a> for Input {
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
        Ok(Input {
            // TODO: Try other ranges?
            width: u.int_in_range(0..=4096)?,
            height: u.int_in_range(0..=4096)?,
            // TODO: How to handle zero?
            block_height: u.int_in_range(1..=32)?,
            bytes_per_pixel: u.int_in_range(1..=16)?,
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
            input.block_height,
            input.bytes_per_pixel
        )
    ];

    let mut deswizzled = vec![
        0u8;
        nutexb_swizzle::deswizzled_surface_size(
            input.width,
            input.height,
            input.bytes_per_pixel
        )
    ];

    nutexb_swizzle::deswizzle_block_linear(
        input.width,
        input.height,
        &swizzled,
        &mut deswizzled,
        input.block_height,
        input.bytes_per_pixel,
    );
});
