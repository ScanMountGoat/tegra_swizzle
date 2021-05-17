use binwrite::BinWrite;
use std::io::Write;
#[derive(BinWrite)]
struct NutexbFile {
    data: Vec<u8>,
    footer: NutexbFooter,
}

#[derive(BinWrite)]
struct NutexbFooter {
    #[binwrite(align_after(0x40))]
    mip_sizes: Vec<u32>,

    string_magic: [u8; 4],

    #[binwrite(align_after(0x40))]
    string: String,

    #[binwrite(pad(4))]
    width: u32,
    height: u32,
    depth: u32,

    image_format: u8,

    #[binwrite(pad_after(0x2))]
    unk: u8, // 4?

    unk2: u32,
    mip_count: u32,
    alignment: u32,
    array_count: u32,
    size: u32,

    tex_magic: [u8; 4],
    version_stuff: (u16, u16),
}

pub fn write_nutexb_from_data<W: Write>(
    writer: &mut W,
    data: &[u8],
    width: u32,
    height: u32,
    name: &str,
    image_format: u8,
) -> std::io::Result<()> {
    let size = data.len() as u32;
    NutexbFile {
        data: data.into(),
        footer: NutexbFooter {
            mip_sizes: vec![size as u32],
            string_magic: *b" XNT",
            string: name.into(),
            width,
            height,
            depth: 1,
            image_format,
            unk: 4,
            unk2: 4,
            mip_count: 1,
            alignment: 0x1000,
            array_count: 1,
            size,
            tex_magic: *b" XET",
            version_stuff: (1, 2),
        },
    }
    .write(writer)
}
