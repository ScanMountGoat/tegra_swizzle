# Swizzling
## Introduction
The Nutexb/Tegra X1 texture swizzling can be described as a swizzle function `swizzle: L -> S` where `L` is the set of linear input addresses and `S` is the set of swizzled output addresses. The function `swizzle` maps or "swizzles" a pixel address in `L` to some corresponding address in `S`. The operation of mapping swizzled addresses in `S` to their original linear address in `L` is called "deswizzling".

The function `swizzle` is injective, meaning that if `a1` and `a2` are distinct addresses in `L`, their corresponding output addresses `swizzle(a1)` and `swizzle(a2)` in `S` are distinct. If this were not the case, two different pixels could be mapped to the same swizzled pixel location, causing information loss. This implies that the set of linear addresses `L` must be at least as large as `S`. It might be the case that `S` has more elements than `L` due to padding or some other constraint, so deswizzling needs to be defined slightly more carefully. These "unmapped" elements are padding bytes and can be safely set to zero. The function `swizzle` is not invertible since some elements in `S` have no corresponding element in `L`. It is still possible to define a function `deswizzle: S -> L` by looking up the corresponding address in `S` for each address in `L`. The "padding" bytes that don't appear in the mapping are never read for deswizzling.

This means that only the swizzle function `swizzle` needs to be defined. For a specific pair of swizzled and deswizzled images, this transformation can be represented as a lookup table for input and output addresses. See the [swizzle_data](https://github.com/ScanMountGoat/nutexb_swizzle/tree/main/swizzle_data) for input output pairs for swizzling and deswizzling.  

In the case where the swizzled and deswizzled surface sizes in bytes are the same, the function `swizzle` is also bijective. Being bijective means that each input address is mapped to a unique output address. This also implies the sets `swizzle` and `S` have the same number of elements. `swizzle` and `S` have the same size, and no two inputs are mapped to the same output, so it's possible to perform swizzling and deswizzling in place without any memory allocations. This happens rarely in practice due to padding and alignment of swizzled surfaces.

For the power of two case, `swizzle` can be represented with drastically less memory using bit patterns for the x and y components of the address. See the [swizzling blog post](https://fgiesen.wordpress.com/2011/01/17/texture-tiling-and-swizzling/) for details.

## Implementations
The evolution of techniques used for this repository are listed below. Note that later techniques tend to add additional complexity but generalize to more inputs.
1. lookup tables for specific width, height, and bytes per pixel values.
2. generated bit patterns as a more efficient encoding of lookup tables for power of two textures
3. A naive implementation of the swizzle function defined in the Tegra X1 TRM. Note that this is defined over byte coordinates x,y and not pixel coordinates. This means the same 
code can be applied to arbitrary formats since their block sizes and bytes per pixel simply define the transformation from pixel to byte coordinates.
4. An optimized implementation of the swizzle function that compiles to SIMD instructions on supported platforms. The current implementation applies an optimized loop over 64x8 byte tiles (GOBs) and uses the naive implementation to handle the remaining bytes. 

## Swizzle Patterns
Nutexb formats have either 4 bytes, 8 bytes, or 16 bytes per block. For uncompressed formats, pixels are treated as blocks.

### Block 4 Swizzle Patterns (R8G8B8A8, B8G8R8A8)
Swizzle patterns to find the corresponding pixel address when swizzing or deswizzling.  
The starting address of each block requires padding the address on the right with 2 bits since RGBA pixels are 4 bytes.  

| Width | Height | X Pattern | Y Pattern |
| --- | --- | --- | ------- |
| 64   | 64   | 0000000111000111 | 00000000111000111000 |
| 128  | 128  | 0000001110001111 | 00000011110001110000 |
| 256  | 256  | 0000111100001111 | 00001111000011110000 |
| 512  | 512  | 0011111000001111 | 00111100000111110000 |
| 1024 | 1024 | 1111110000001111 | 11110000001111110000 |

### Block 8 Swizzle Patterns (BC1, BC4)
Swizzle patterns to find the corresponding block index when swizzling or deswizzling.  
Each 4x4 tile is represented as a single 8 byte block. The starting byte address of each block requires 
padding the address on the right with 3 bits since blocks are 8 bytes.  

| Width (pixels) | Height (pixels) | Width (tiles) | Height (tiles) | X Pattern | Y Pattern |
| --- | --- | --- | --- | --- | --- |
| 8   | 8   | 2   | 2   | 00000000000001 | 000000000000010 |
| 16  | 16  | 4   | 4   | 00000000000101 | 000000000001010 |
| 32  | 32  | 8   | 8   | 00000000100101 | 000000000011010 |
| 64  | 64  | 16  | 16  | 00000010100101 | 000000001011010 |
| 128 | 128 | 32  | 32  | 00001100100101 | 000000011011010 |
| 256 | 256 | 64  | 64  | 00111000100101 | 000000111011010 |
| 512 | 512 | 128 | 128 | 11110000100101 | 000001111011010 |

### Block 16 Swizzle Patterns (BC2, BC3, BC5, BC6, BC7, R32G32B32A32_Float)
Swizzle patterns to find the corresponding block index when swizzling or deswizzling.  
Each 4x4 tile is represented as a single 16 byte block. The starting byte address of each block requires 
padding the address on the right with 4 bits since blocks are 16 bytes.  

R32G32B32A32_Float uses the same swizzle patterns since each pixel also requires 16 bytes. 
Since this is not a compressed format, use the tile width and tile height in the table instead. 
For example, a 128x128 pixel R32G32B32A32_Float image will use the patterns for 512, 512, 128, 128 in the table below.

| Width (pixels) | Height (pixels) | Width (tiles) | Height (tiles) | X Pattern | Y Pattern |
| --- | --- | --- | --- | --- | --- |
| 8    | 8    | 2    | 2    | 00000000000000100010 | 00000000000000100001 |
| 16   | 16   | 4    | 4    | 00000000000000010010 | 00000000000000000101 |
| 32   | 32   | 8    | 8    | 00000000000000110010 | 00000000000000001101 |
| 64   | 64   | 16   | 16   | 00000000000011010010 | 00000000000000101101 |
| 128  | 128  | 32   | 32   | 00000000001110010010 | 00000000000001101101 |
| 256  | 256  | 64   | 64   | 00000000111100010010 | 00000000000011101101 |
| 512  | 512  | 128  | 128  | 00000011111000010010 | 00000000000111101101 |
| 1024 | 1024 | 256  | 256  | 00000111111000010010 | 00001000000111101101 |
| 2048 | 2048 | 512  | 512  | 00001111111000010010 | 00110000000111101101 |
| 4096 | 4096 | 1024 | 1024 | 00011111110000000111 | 11100000001111111000 |

