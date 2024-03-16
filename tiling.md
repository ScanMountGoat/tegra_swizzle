# Memory Tiling
## Introduction
The Nutexb/Tegra X1 texture tiling can be described as a function `tile: L -> S` where `L` is the set of linear input addresses and `T` is the set of tiled output addresses. The function `tile` maps or "tiles" a pixel address in `L` to some corresponding address in `T`. The operation of mapping tiled addresses in `T` to their original linear address in `L` is called "untiling".

The function `tile` is injective, meaning that if `l1` and `l2` are distinct addresses in `L`, their corresponding output addresses `tile(l1)` and `tile(l2)` in `T` are distinct. If this were not the case, two different pixels could be mapped to the same tiled pixel location, causing information loss. This implies that the set of tiled addresses `T` must be at least as large as `L`. 

It might be the case that `T` has more elements than `L` due to padding or some other constraint, so untiling needs to be defined slightly more carefully. These "unmapped" elements are padding bytes and can be safely set to zero. The function `tile` is not invertible since some elements in `T` have no corresponding element in `L`. It is still possible to define a function `detile: T -> L` by looking up the corresponding address in `T` for each address in `L`. The "padding" bytes that don't appear in the mapping are never read for untiling.

This means that only the function `tile` needs to be explicitly defined. For a specific pair of tiled and detiled images, this transformation can be represented as a lookup table for input and output addresses. See the [tile_data](https://github.com/ScanMountGoat/tegra_swizzle/tree/main/tile_data) for input output pairs for tiling and untiling.  

In the case where the tiled and detiled surface sizes in bytes are the same, the function `tile` is also bijective. Being bijective means that each input address is mapped to a unique output address. This also implies the sets `L` and `T` have the same number of elements. `L` and `T` have the same size, and no two inputs are mapped to the same output, so it's possible to perform tiling and untiling in place without any memory allocations. This happens rarely in practice due to padding and alignment of tiled surfaces.

## Implementations
The evolution of techniques used for this repository are listed below. Note that later techniques tend to add additional complexity but generalize to more inputs.
1. Lookup tables for specific width, height, and bytes per pixel values.
2. Generated [bit patterns](https://fgiesen.wordpress.com/2011/01/17/texture-tiling-and-tiling/) as a more efficient encoding of lookup tables for power of two textures.
3. A naive implementation of the tile function defined in the Tegra X1 TRM. Note that this is defined over byte coordinates x,y and not pixel coordinates. This means the same code can be applied to arbitrary formats since their block sizes and bytes per pixel simply define the transformation from pixel to byte coordinates.
4. An optimized implementation of the tile function that compiles to SIMD instructions on supported platforms. The current implementation applies an optimized loop over 64x8 byte tiles (GOBs) and uses the naive implementation to handle the remaining bytes. 