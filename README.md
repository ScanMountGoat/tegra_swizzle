# nutexb_swizzle
Documentation and tools for Tegra X1 swizzling used for nutexb texture files for Smash Ultimate. See the [swizzle](swizzle.md) page for documentation.

## Generating Test Data
1. Write unique block values to the Nutexb. Pad the image size with `--blockcount` as needed.
`cargo run -- write_addresses -w 512 -h 512 -f bc7 -o "def_mario_001_col.nutexb" --blockcount 43872` 
2. Write the unique block values to a binary file. Pad the image size with `--blockcount` as needed.
`cargo run -- write_addresses -w 512 -h 512 -f bc7 -o "/swizzle_data/linear.bin" --blockcount 43872`
3. Calculate the deswizzled version of `linear.bin` using an emulator or reference swizzle/deswizzle implementation. Save the resulting blocks to `deswizzle.bin`.
4. For power of two textures, guess the swizzle pattern.
`cargo run -- calculate_swizzle -w 512 -h 512 -f bc7 --swizzled "linear.bin" --deswizzled "deswizzle.bin"`