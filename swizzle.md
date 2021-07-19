# Swizzling
The Nutexb/Tegra X1 texture swizzling can be described as a swizzle function `S: I -> O` where `I` is the set of linear input addresses and `O` is the set of swizzled output addresses. The function `S` maps or "swizzles" a pixel address in `I` to some corresponding address in `O`. The operation of mapping swizzled addresses in `O` to their original linear address in `I` is called "deswizzling". 

In the case where the texture is square and both dimensions are a power of two, the function `S` is bijective. Being bijective means that each input address is mapped to a unique output address. This also implies the sets `S` and `O` have the same number of elements. `S` and `O` have the same size, and no two inputs are mapped to the same output, so it's possible to perform swizzling and deswizzling in place without any memory allocations.  

The case for general texture dimensions is not as trivial. The function `S` is still injective, meaning that if `a1` and `a2` are distinct addresses in `I`, their corresponding output addresses `S(a1)` and `S(a2)` in `O` are distinct. If this were not the case, two different pixels could be mapped to the same swizzled pixel location, causing information loss. This implies that `I` must have at least as many elements as `O`. The function `S` is not invertible since it is not bijective, but it's still possible to define a "deswizzle" function by mapping each address in `O` to the corresponding address in `I`. It might be the case that `O` has more elements than `I` due to padding or some other constraint, so deswizzling needs to be defined slightly more carefully.  

For deswizzling in the general case, the problem of `I` and `O` having a different number of elements can be solved by considering the elements of `I`. Rather than mapping swizzled addresses to linear addresses, we can map linear addresses to swizzled addresses since each linear address has a corresponding swizzled address since `S` is injective. This means the function `S` can be reused to perform both swizzling and deswizzling by swapping the input addresses. In this case, the swizzling and deswizzling only requires a single allocation for the output.  

This means that only the swizzle function `S` needs to be defined for the appropriate formats and dimensions. The swizzle function depends on the width in elements, height in elements, and size of each element. The elements can be defined to be pixels, blocks, or tiles as long as the width and height are calculated appropriately. 

In the general case, this transformation can be represented as a lookup table for input and output addresses. Creating an efficient way to compute the non power of two case is still a work in progress. See the [swizzle_data](https://github.com/ScanMountGoat/nutexb_swizzle/tree/main/swizzle_data) for input output pairs for deswizzling. The `..._linear.bin` files are the input files and the `..._linear_deswizzle.bin` files are the result of deswizzling the input.

For the power of two case, `S` can be represented more efficiently as bit patterns for the x and y components of the address. See the [swizzling blog post](https://fgiesen.wordpress.com/2011/01/17/texture-tiling-and-swizzling/) for details.



## R8G8B8A8, B8G8R8A8 Pixel Swizzle Patterns 
Swizzle patterns to find the corresponding pixel address when swizzing or deswizzling.  
The starting address of each block requires padding the address on the right with 2 bits since RGBA pixels are 4 bytes.  

| Width | Height | X Pattern | Y Pattern |
| --- | --- | --- | ------- |
| 64  | 64  | 0000000111000111 | 000000111000111000 |
| 128 | 128 | 0000001110001111 | 000011110001110000 |
| 256 | 256 | 0000111100001111 | 001111000011110000 |
| 512 | 512 | 0011111000001111 | 111100000111110000 |

## BC2, BC3, BC5, BC6, BC7, R32G32B32A32_Float Swizzle Patterns 
Swizzle patterns to find the corresponding block index when swizzling or deswizzling.  
Each 4x4 tile is represented as a single 16 byte block. The starting byte address of each block requires 
padding the address on the right with 4 bits since blocks are 16 bytes.  

R32G32B32A32_Float uses the same swizzle patterns since each pixel also requires 16 bytes. 
Since this is not a compressed format, use the tile width and tile height in the table instead. 
For example, a 128x128 pixel R32G32B32A32_Float image will use the patterns for 512, 512, 128, 128 in the table below.

*TODO: Investigate sizes smaller than 16x16*

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


## BC1, BC4 Swizzle Patterns 
Swizzle patterns to find the corresponding block index when swizzling or deswizzling.  
Each 4x4 tile is represented as a single 8 byte block. The starting byte address of each block requires 
padding the address on the right with 3 bits since blocks are 8 bytes.  

*TODO: Investigate sizes smaller than 16x16*

| Width (pixels) | Height (pixels) | Width (tiles) | Height (tiles) | X Pattern | Y Pattern |
| --- | --- | --- | --- | --- | --- |
| 8   | 8   | 2   | 2   | 00000000000001 | 000000000000010 |
| 16  | 16  | 4   | 4   | 00000000000101 | 000000000001010 |
| 32  | 32  | 8   | 8   | 00000000100101 | 000000000011010 |
| 64  | 64  | 16  | 16  | 00000010100101 | 000000001011010 |
| 128 | 128 | 32  | 32  | 00001100100101 | 000000011011010 |
| 256 | 256 | 64  | 64  | 00111000100101 | 000000111011010 |
| 512 | 512 | 128 | 128 | 11110000100101 | 000001111011010 |
