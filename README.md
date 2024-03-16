# tegra_swizzle 
<img src="https://raw.githubusercontent.com/ScanMountGoat/tegra_swizzle/main/tiled3d.png" height="auto" width="100%">
<img src="https://raw.githubusercontent.com/ScanMountGoat/tegra_swizzle/main/linear3d.png" height="auto" width="100%">

[![Latest Version](https://img.shields.io/crates/v/tegra_swizzle.svg)](https://crates.io/crates/tegra_swizzle) [![docs.rs](https://docs.rs/tegra_swizzle/badge.svg)](https://docs.rs/tegra_swizzle)

A safe and efficient pure Rust implementation of the block linear memory layout algorithm used for texture surfaces for the Tegra X1 in the Nintendo Switch.

The above image shows a tiled RGBA 16x16x16 pixel 3D lut. The different colored blocks correspond to a 16x2 grid of GOBs ("groups of bytes" from the Tegra TRM). GOBs are 64x8 bytes (512 total bytes), which in this case is 16x8 pixels. The untiled or linear version is shown below.

## Memory Tiling
GPU textures are often stored in a tiled memory layout to make texture accesses more cache friendly. The standard linear or row-major memory ordering is only cache friendly when the data is accessed in row-major order. This is rarely the case for image textures for models, so the bytes of a surface are rearranged to improve the number of cache misses using some form of tiling algorithm. 

It's important to note that tiling affects memory addressing, so surfaces should be thought of as 2D or 3D arrays of bytes rather than pixels or 4x4 pixel blocks. The tiling algorithm is agnostic to whether the data is RGBA8 or BC7 compressed. The format is used only for converting the surface dimensions from pixels to bytes. This avoids making assumptions about relationships between the size of a pixel or compressed block and the tiling algorithm and results in more efficient code. The byte dimensions of tiled surfaces are rounded up to integral dimensions in GOBs (64x8) bytes. The surface dimensions in pixels do not need to be powers of two for tiling to work correctly.

This technique has often been referred to in Switch modding communities as "swizzling", "deswizzling", "unswizzling", or "un-swizzling". It's not accurate to describe the block linear addresses as rearranged or "swizzled" from linear addresses for all texture sizes. Thankfully, common usages of the term "swizzling" in modding communities almost always refer specifically to the block linear memory layout algorithm. The term "swizzling" is kept in crate and function names to improve discoverability as this is likely what most programmers will search for.

## Building
For using the library in other languages through C FFI, first build the library with `cargo build --release --features=ffi`. This requires the Rust toolchain to be installed. The generated `tegra_swizzle.dll` or `tegra_swizzle.so` depending on the platform can be used the same way as any other compiled C library. See the ffi module in the docs.rs link for documentation. 

For building plugins for the Nintendo Switch, see [skyline](https://github.com/ultimate-research/skyline-rs).

## Test Data
This repository contains [sample data](https://github.com/ScanMountGoat/tegra_swizzle/tree/main/block_linear) for testing tiling and untiling. These files were generated using the implementation for Ryujinx emulator due to difficulties in testing on actual hardware. For additional tests used by tegra_swizzle, see the source code and fuzz directories.   

## Documentation
See the [tiling](tiling.md) page for a more formal description of tiling. While not rigorous enough to be considered a proof, this helps motivate some of the techniques and optimizations applied to this library. The [tiling and swizzling blog post](https://fgiesen.wordpress.com/2011/01/17/texture-tiling-and-swizzling/) also provides some additional insights. Note that tegra_swizzle does not use the bit interleaving trick described in the blog post.
