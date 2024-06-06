#![no_main]
use libfuzzer_sys::fuzz_target;

extern crate arbitrary;
use arbitrary::{Arbitrary, Result, Unstructured};

extern crate rand;
use rand::{rngs::StdRng, Rng, SeedableRng};

use tegra_swizzle::surface::BlockDim;

#[derive(Debug)]
struct Input {
    width: u32,
    height: u32,
    depth: u32,
    block_height: tegra_swizzle::BlockHeight,
    bytes_per_pixel: u32,
    layer_count: u32,
    mipmap_count: u32,
}

impl<'a> Arbitrary<'a> for Input {
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
        Ok(Input {
            width: u.int_in_range(0..=256)?,
            height: u.int_in_range(0..=256)?,
            depth: u.int_in_range(0..=256)?,
            block_height: u.arbitrary()?,
            bytes_per_pixel: u.int_in_range(0..=32)?,
            layer_count: u.int_in_range(0..=12)?,
            mipmap_count: u.int_in_range(0..=32)?,
        })
    }
}

fuzz_target!(|input: Input| {
    let deswizzled_size = tegra_swizzle::surface::deswizzled_surface_size(
        input.width,
        input.height,
        input.depth,
        BlockDim::uncompressed(),
        input.bytes_per_pixel,
        input.mipmap_count,
        input.layer_count,
    );

    let seed = [13u8; 32];
    let mut rng: StdRng = SeedableRng::from_seed(seed);
    let deswizzled: Vec<_> = (0..deswizzled_size)
        .map(|_| rng.gen_range::<u8, _>(0..=255))
        .collect();

    let swizzled = tegra_swizzle::surface::swizzle_surface(
        input.width,
        input.height,
        input.depth,
        &deswizzled,
        BlockDim::uncompressed(),
        Some(input.block_height),
        input.bytes_per_pixel,
        input.mipmap_count,
        input.layer_count,
    )
    .unwrap();

    let new_deswizzled = tegra_swizzle::surface::deswizzle_surface(
        input.width,
        input.height,
        input.depth,
        &swizzled,
        BlockDim::uncompressed(),
        Some(input.block_height),
        input.bytes_per_pixel,
        input.mipmap_count,
        input.layer_count,
    )
    .unwrap();

    if deswizzled != new_deswizzled {
        panic!("Swizzle deswizzle is not 1:1");
    }
});
