# Swizzling

## Uncompressed R8G8B8A8, B8G8R8A8 Pixel Swizzle Patterns 
Swizzle patterns to find the corresponding pixel address when swizzing or deswizzling.  
The starting address of each block requires padding the address on the right with 2 bits since RGBA pixels are 4 bytes.  

| Width | Height | X Pattern | Y Pattern |
| --- | --- | --- | ------- |
| 64  | 64  | 0000110001001011 | 0000001110110100 |
| 256 | 256 | 0111100001001011 | 1000011110110100 |

## Compressed BC2, BC3, BC5, BC6, BC7 Block Swizzle Patterns 
Swizzle patterns to find the corresponding block index when swizzling or deswizzling.  
Each 4x4 tile is represented as a single 16 byte block. The starting byte address of each block requires 
padding the address on the right with 4 bits since blocks are 16 bytes.  

*TODO: Investigate sizes smaller than 16x16*

| Width (pixels) | Height (pixels) | Width (tiles) | Height (tiles) | X Pattern | Y Pattern |
| --- | --- | --- | --- | --- | --- |
| 8    | 8    | 2   | 2   | 000000000100010 | 0000000000100001 |
| 16   | 16   | 4   | 4   | 000000000010010 | 0000000000000101 |
| 32   | 32   | 8   | 8   | 000000000110010 | 0000000000001101 |
| 64   | 64   | 16  | 16  | 000000011010010 | 0000000000101101 |
| 128  | 128  | 32  | 32  | 000001110010010 | 0000000001101101 |
| 256  | 256  | 64  | 64  | 000111100010010 | 0000000011101101 |
| 512  | 512  | 128 | 128 | 011111000010010 | 0000000111101101 |
| 1024 | 1024 | 256 | 256 | 111111000010010 | 1000000111101101 |

## Compressed BC1, BC4 Block Swizzle Patterns 
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
