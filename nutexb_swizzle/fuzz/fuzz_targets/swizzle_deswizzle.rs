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
            // TODO: Handle different bpps?
            bytes_per_pixel: 4,
        })
    }
}

fuzz_target!(|input: Input| {
    // fuzzed code goes here
    let deswizzled_size =
        nutexb_swizzle::deswizzled_surface_size(input.width, input.height, input.bytes_per_pixel);
    let deswizzled: Vec<u8> = (0..deswizzled_size as u32).flat_map(|i| i.to_le_bytes()).collect();

    let mut swizzled = vec![
        0u8;
        nutexb_swizzle::swizzled_surface_size(
            input.width,
            input.height,
            input.block_height,
            input.bytes_per_pixel
        )
    ];

    nutexb_swizzle::swizzle_block_linear(
        input.width,
        input.height,
        &deswizzled,
        &mut swizzled,
        input.block_height,
        input.bytes_per_pixel,
    );

    let mut new_deswizzled = vec![0u8; deswizzled_size];
    nutexb_swizzle::deswizzle_block_linear(
        input.width,
        input.height,
        &swizzled,
        &mut new_deswizzled,
        input.block_height,
        input.bytes_per_pixel,
    );

    if deswizzled != new_deswizzled {
        panic!("Swizzle deswizzle is not 1:1");
    }
    // assert_eq!(deswizzled, new_deswizzled, "");
});
