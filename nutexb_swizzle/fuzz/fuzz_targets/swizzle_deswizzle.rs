#![no_main]
use libfuzzer_sys::fuzz_target;

extern crate arbitrary;
use arbitrary::{Arbitrary, Result, Unstructured};

extern crate rand;
use rand::{Rng, SeedableRng, rngs::StdRng};

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
            width: u.int_in_range(0..=4096)?,
            height: u.int_in_range(0..=4096)?,
            depth: 1, // TODO: Test other depths?
            block_height: u.arbitrary()?,
            bytes_per_pixel: u.int_in_range(0..=32)?,
        })
    }
}

fuzz_target!(|input: Input| {
    let deswizzled_size =
        nutexb_swizzle::deswizzled_surface_size(input.width, input.height, input.depth, input.bytes_per_pixel);

    let seed = [13u8; 32];
    let mut rng: StdRng = SeedableRng::from_seed(seed);
    let deswizzled: Vec<_> = (0..deswizzled_size).map(|_| rng.gen_range::<u8, _>(0..=255)).collect();

    let swizzled = nutexb_swizzle::swizzle_block_linear(
        input.width,
        input.height,
        input.depth,
        &deswizzled,
        input.block_height,
        input.bytes_per_pixel,
    ).unwrap();

    let new_deswizzled = nutexb_swizzle::deswizzle_block_linear(
        input.width,
        input.height,
        input.depth,
        &swizzled,
        input.block_height,
        input.bytes_per_pixel,
    ).unwrap();

    if deswizzled != new_deswizzled {
        panic!("Swizzle deswizzle is not 1:1");
    }
});
