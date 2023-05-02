# tegra_swizzle 
<img src="https://raw.githubusercontent.com/ScanMountGoat/nutexb_swizzle/main/swizzle3d.png" height="auto" width="100%">
<img src="https://raw.githubusercontent.com/ScanMountGoat/nutexb_swizzle/main/deswizzled3d.png" height="auto" width="100%">

[![Latest Version](https://img.shields.io/crates/v/tegra_swizzle.svg)](https://crates.io/crates/tegra_swizzle) [![docs.rs](https://docs.rs/tegra_swizzle/badge.svg)](https://docs.rs/tegra_swizzle)

A safe and efficient pure Rust implementation of swizzling and deswizzling for the for the Tegra X1 block linear swizzling algorithm used for the Nintendo Switch.

The above image shows a swizzled RGBA 16x16x16 pixel 3D lut. The different colored blocks correspond to a 16x2 grid of GOBs ("groups of bytes" from the Tegra TRM). GOBs are 64x8 bytes (512 total bytes), which in this case is 16x8 pixels. The deswizzled version is shown below.

## Swizzling
Texture are often stored in a swizzled layout to make texture accesses more cache friendly. The standard linear or row-major memory ordering is only cache friendly when the data is accessed in row-major order. This is rarely the case for image textures for models, so the bytes of a surface are rearranged to improve the number of cache misses using a process known as "swizzling". 

It's important to note that swizzling affects memory addressing, so surfaces should be thought of as 2D or 3D arrays of bytes rather than pixels or 4x4 pixel blocks. The swizzling algorithm is agnostic to whether the data is RGBA8 or BC7 compressed. The format is used only for converting the surface dimensions from pixels to bytes. This avoids making assumptions about relationships between the size of a pixel or compressed block and the swizzling algorithm and results in more efficient code. The byte dimensions of swizzled surfaces are rounded up to integral dimensions in GOBs (64x8) bytes. The surface dimensions in pixels do not need to be powers of two for swizzling to work correctly.

## C FFI 
For using the library in other languages through C FFI, first build the library with `cargo build --release --features=ffi`. This requires the Rust toolchain to be installed.  

The generated `tegra_swizzle.dll` or `tegra_swizzle.so` depending on the platform can be used the same way as any other compiled C library. See the ffi module in the docs.rs link for documentation.

## Test Data
This repository contains [sample data](https://github.com/ScanMountGoat/nutexb_swizzle/tree/main/swizzle_data) for testing swizzling and deswizzling. These files were generated using the swizzling implementation for Ryujinx emulator due to difficulties in testing on actual hardware. For additional tests used by tegra_swizzle, see the source code and fuzz directories.   

## Documentation
See the [swizzle](swizzle.md) page for a more formal description of swizzling. While not rigorous enough to be considered a proof, this helps motivate some of the techniques and optimizations applied to this library. The following [swizzling blog post](https://fgiesen.wordpress.com/2011/01/17/texture-tiling-and-swizzling/) also provides some additional insights into swizzling. Note that the technique used in this implementation differs from those described in the post.