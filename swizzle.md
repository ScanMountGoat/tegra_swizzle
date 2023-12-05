# Swizzling
## Introduction
The Nutexb/Tegra X1 texture swizzling can be described as a swizzle function `swizzle: L -> S` where `L` is the set of linear input addresses and `S` is the set of swizzled output addresses. The function `swizzle` maps or "swizzles" a pixel address in `L` to some corresponding address in `S`. The operation of mapping swizzled addresses in `S` to their original linear address in `L` is called "deswizzling".

The function `swizzle` is injective, meaning that if `l1` and `l2` are distinct addresses in `L`, their corresponding output addresses `swizzle(l1)` and `swizzle(l2)` in `S` are distinct. If this were not the case, two different pixels could be mapped to the same swizzled pixel location, causing information loss. This implies that the set of swizzled addresses `S` must be at least as large as `L`. 

It might be the case that `S` has more elements than `L` due to padding or some other constraint, so deswizzling needs to be defined slightly more carefully. These "unmapped" elements are padding bytes and can be safely set to zero. The function `swizzle` is not invertible since some elements in `S` have no corresponding element in `L`. It is still possible to define a function `deswizzle: S -> L` by looking up the corresponding address in `S` for each address in `L`. The "padding" bytes that don't appear in the mapping are never read for deswizzling.

This means that only the swizzle function `swizzle` needs to be defined. For a specific pair of swizzled and deswizzled images, this transformation can be represented as a lookup table for input and output addresses. See the [swizzle_data](https://github.com/ScanMountGoat/nutexb_swizzle/tree/main/swizzle_data) for input output pairs for swizzling and deswizzling.  

In the case where the swizzled and deswizzled surface sizes in bytes are the same, the function `swizzle` is also bijective. Being bijective means that each input address is mapped to a unique output address. This also implies the sets `swizzle` and `S` have the same number of elements. `swizzle` and `S` have the same size, and no two inputs are mapped to the same output, so it's possible to perform swizzling and deswizzling in place without any memory allocations. This happens rarely in practice due to padding and alignment of swizzled surfaces.

## Implementations
The evolution of techniques used for this repository are listed below. Note that later techniques tend to add additional complexity but generalize to more inputs.
1. Lookup tables for specific width, height, and bytes per pixel values.
2. Generated [bit patterns](https://fgiesen.wordpress.com/2011/01/17/texture-tiling-and-swizzling/) as a more efficient encoding of lookup tables for power of two textures.
3. A naive implementation of the swizzle function defined in the Tegra X1 TRM. Note that this is defined over byte coordinates x,y and not pixel coordinates. This means the same code can be applied to arbitrary formats since their block sizes and bytes per pixel simply define the transformation from pixel to byte coordinates.
4. An optimized implementation of the swizzle function that compiles to SIMD instructions on supported platforms. The current implementation applies an optimized loop over 64x8 byte tiles (GOBs) and uses the naive implementation to handle the remaining bytes. 