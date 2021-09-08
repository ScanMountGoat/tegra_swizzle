use std::str::FromStr;

/// Supported Nutexb image formats for swizzle operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    Rgba8,
    RgbaF32,
    Bc1,
    Bc3,
    Bc7,
}

impl FromStr for ImageFormat {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "rgba8" => Ok(ImageFormat::Rgba8),
            "rgbaf32" => Ok(ImageFormat::RgbaF32),
            "bc1" => Ok(ImageFormat::Bc1),
            "bc3" => Ok(ImageFormat::Bc3),
            "bc7" => Ok(ImageFormat::Bc7),
            _ => Err("Unsupported format"),
        }
    }
}

impl ImageFormat {
    /// Gets the size of a single tile in bytes for block compressed formats
    /// or the bytes per pixel for uncompressed formats.
    pub const fn tile_size_in_bytes(&self) -> usize {
        match self {
            ImageFormat::Rgba8 => 4,
            ImageFormat::Bc1 => 8,
            ImageFormat::Bc3 | ImageFormat::Bc7 | ImageFormat::RgbaF32 => 16,
        }
    }

    /// Gets the number of tiles needed to represent the given dimensions.
    /// Uncompressed formats are assumed to have single pixel tiles.
    pub const fn tile_count(&self, width: usize, height: usize) -> usize {
        // TODO: This should round up for non multiples of the tile size.
        let tile_dimension = self.tile_dimension();
        width * height / (tile_dimension * tile_dimension)
    }

    /// Gets the number of pixels for the width or height of a square tile.
    /// Uncompressed formats are assumed to have single pixel tiles.
    pub const fn tile_dimension(&self) -> usize {
        match self {
            ImageFormat::Rgba8 | ImageFormat::RgbaF32 => 1,
            _ => 4,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tile_sizes() {
        assert_eq!(4, ImageFormat::tile_size_in_bytes(&ImageFormat::Rgba8));
        assert_eq!(8, ImageFormat::tile_size_in_bytes(&ImageFormat::Bc1));
        assert_eq!(16, ImageFormat::tile_size_in_bytes(&ImageFormat::Bc3));
        assert_eq!(16, ImageFormat::tile_size_in_bytes(&ImageFormat::Bc7));
        assert_eq!(
            16,
            ImageFormat::tile_size_in_bytes(&ImageFormat::RgbaF32)
        );
    }

    #[test]
    fn tile_counts() {
        assert_eq!(80, ImageFormat::tile_count(&ImageFormat::Rgba8, 5, 16));
        assert_eq!(5, ImageFormat::tile_count(&ImageFormat::Bc1, 5, 16));
        assert_eq!(5, ImageFormat::tile_count(&ImageFormat::Bc3, 5, 16));
        assert_eq!(5, ImageFormat::tile_count(&ImageFormat::Bc7, 5, 16));
        assert_eq!(
            80,
            ImageFormat::tile_count(&ImageFormat::RgbaF32, 5, 16)
        );
    }

    #[test]
    fn tile_dimensions() {
        assert_eq!(1, ImageFormat::tile_dimension(&ImageFormat::Rgba8));
        assert_eq!(4, ImageFormat::tile_dimension(&ImageFormat::Bc1));
        assert_eq!(4, ImageFormat::tile_dimension(&ImageFormat::Bc3));
        assert_eq!(4, ImageFormat::tile_dimension(&ImageFormat::Bc7));
        assert_eq!(1, ImageFormat::tile_dimension(&ImageFormat::RgbaF32));
    }
}
