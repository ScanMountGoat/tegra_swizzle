# Swizzling

## Uncompressed R8G8B8A8 Pixel Swizzle Patterns 
Swizzle patterns to find the corresponding pixel address when swizzing or deswizzling.  
The starting address of each block requires padding the address on the right with 2 bits since RGBA pixels have 4 bytes.  

| Width | Height | X Pattern | Y Pattern |
| --- | --- | --- | ------- |
| 64  | 64  | 0000110001001011 | 0000001110110100 |
| 256 | 256 | 0111100001001011 | 1000011110110100 |

## Compressed BC3, BC7 Block Swizzle Patterns 
Swizzle patterns to find the corresponding block index when swizzling or deswizzling.  
Each 4x4 tile is represented as a single 16 byte block. The starting address of each block requires 
padding the address on the right with 4 bits since blocks have 16 bytes.  

| Width (pixels) | Height (pixels) | Width (tiles) | Height (tiles) | X Pattern | Y Pattern |
| --- | --- | --- | --- | --- | --- |
| 64  | 64  | 16  | 16  | 00000011010010 | 00000000101101 |
| 128 | 128 | 32  | 32  | 00001110010010 | 00000001101101 |
| 256 | 256 | 64  | 64  | 00111100010010 | 00000011101101 |
| 512 | 512 | 128 | 128 | 11111000010010 | 00000111101101 |

## Compressed BC1 Block Swizzle Patterns 
Swizzle patterns to find the corresponding block index when swizzling or deswizzling.  
Each 4x4 tile is represented as a single 8 byte block. The starting address of each block requires 
padding the address on the right with 3 bits since blocks have 16 bytes.  

| Width (pixels) | Height (pixels) | Width (tiles) | Height (tiles) | X Pattern | Y Pattern |
| --- | --- | --- | --- | --- | --- |
| 128 | 128 | 32 | 32 | 001100100101 | 000011011010 |
