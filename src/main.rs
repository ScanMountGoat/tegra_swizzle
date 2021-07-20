use clap::{App, AppSettings, Arg, SubCommand};
use nutexb_swizzle::{deswizzle, swizzle, ImageFormat};
use std::path::Path;

fn main() {
    // TODO: Use a yaml to configure this?
    // TODO: Share common parameters using variables?
    let format_arg = Arg::with_name("format")
        .short("f")
        .long("format")
        .help("The image format")
        .required(true)
        .takes_value(true)
        .possible_values(&["bc1", "bc3", "bc7", "rgba8", "rgbaf32"])
        .case_insensitive(true);

    let image_size_arg = Arg::with_name("imagesize")
        .long("imagesize")
        .help("The total number of bytes of data to write.")
        .required(false)
        .takes_value(true);

    let width_arg = Arg::with_name("width")
        .short("w")
        .long("width")
        .help("The image width in pixels")
        .required(true)
        .takes_value(true);

    let height_arg = Arg::with_name("height")
        .short("h")
        .long("height")
        .help("The image height in pixels")
        .required(true)
        .takes_value(true);

    let matches = App::new("nutexb_swizzle")
        .version("0.1")
        .author("SMG")
        .about("Reverse engineer texture swizzling from generated texture patterns.")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .subcommand(
            SubCommand::with_name("calculate_swizzle")
                .arg(
                    Arg::with_name("swizzled")
                        .long("swizzled")
                        .help("The input swizzled image data. Each block of data should be unique.")
                        .required(true)
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("deswizzled")
                        .long("deswizzled")
                        .help(
                            "The input swizzled data after being deswizzled to linear addressing.",
                        )
                        .required(true)
                        .takes_value(true),
                )
                .arg(&format_arg)
                .arg(&width_arg)
                .arg(&height_arg),
        )
        .subcommand(
            SubCommand::with_name("write_addresses")
                .arg(&format_arg)
                .arg(&width_arg)
                .arg(&height_arg)
                .arg(&image_size_arg)
                .arg(
                    Arg::with_name("output")
                        .short("o")
                        .long("output")
                        .help("The output file for the image data")
                        .required(true)
                        .takes_value(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("swizzle")
                .arg(
                    Arg::with_name("input")
                        .short("i")
                        .long("input")
                        .help("The swizzled input data")
                        .required(true)
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("output")
                        .short("o")
                        .long("output")
                        .help("The deswizzled output data")
                        .required(true)
                        .takes_value(true),
                )
                .arg(&format_arg)
                .arg(&width_arg)
                .arg(&height_arg),
        )
        .subcommand(
            SubCommand::with_name("deswizzle")
                .arg(
                    Arg::with_name("input")
                        .short("i")
                        .long("input")
                        .help("The swizzled input data")
                        .required(true)
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("output")
                        .short("o")
                        .long("output")
                        .help("The deswizzled output data")
                        .required(true)
                        .takes_value(true),
                )
                .arg(&format_arg)
                .arg(&width_arg)
                .arg(&height_arg),
        )
        .get_matches();

    match matches.subcommand() {
        ("write_addresses", Some(sub_m)) => {
            let output = Path::new(sub_m.value_of("output").unwrap());
            let width: usize = sub_m.value_of("width").unwrap().parse().unwrap();
            let height: usize = sub_m.value_of("height").unwrap().parse().unwrap();
            let format =
                nutexb_swizzle::try_get_image_format(sub_m.value_of("format").unwrap()).unwrap();

            // Allow manually overriding the image size.
            let block_count: usize = match sub_m.value_of("imagesize") {
                Some(v) => {
                    let image_size: usize = v.parse().unwrap();
                    // TODO: Make this a shared function.
                    match format {
                        ImageFormat::Rgba8 => image_size / 4,
                        ImageFormat::Bc1 => image_size / 8,
                        ImageFormat::RgbaF32 | ImageFormat::Bc3 | ImageFormat::Bc7 => image_size / 16,
                    }
                }
                None => match format {
                    ImageFormat::Rgba8 => width * height,
                    ImageFormat::RgbaF32 => width * height,
                    _ => width * height / 16,
                },
            };

            let mut writer = std::io::BufWriter::new(std::fs::File::create(output).unwrap());

            if output.extension().unwrap() == "nutexb" {
                // Write the appropriate data to the first miplevel of a new nutexb.
                nutexb_swizzle::create_nutexb(
                    &mut writer,
                    width,
                    height,
                    output
                        .with_extension("")
                        .file_name()
                        .unwrap()
                        .to_str()
                        .unwrap(),
                    &format,
                    block_count,
                );
            } else {
                match format {
                    ImageFormat::Rgba8 => nutexb_swizzle::write_rgba_lut(&mut writer, block_count),
                    ImageFormat::RgbaF32 => {
                        nutexb_swizzle::write_rgba_f32_lut(&mut writer, block_count)
                    }
                    ImageFormat::Bc1 => nutexb_swizzle::write_bc1_lut(&mut writer, block_count),
                    ImageFormat::Bc3 => nutexb_swizzle::write_bc3_lut(&mut writer, block_count),
                    ImageFormat::Bc7 => nutexb_swizzle::write_bc7_lut(&mut writer, block_count),
                }
            };
        }
        ("calculate_swizzle", Some(sub_m)) => {
            let width: usize = sub_m.value_of("width").unwrap().parse().unwrap();
            let height: usize = sub_m.value_of("height").unwrap().parse().unwrap();
            let swizzled_file = sub_m.value_of("swizzled").unwrap();
            let deswizzled_file = sub_m.value_of("deswizzled").unwrap();
            let format =
                nutexb_swizzle::try_get_image_format(sub_m.value_of("format").unwrap()).unwrap();

            let start = std::time::Instant::now();
            match format {
                ImageFormat::Rgba8 => nutexb_swizzle::guess_swizzle_patterns::<u32, _>(
                    swizzled_file,
                    deswizzled_file,
                    width,
                    height,
                    &format,
                ),
                ImageFormat::Bc1 => nutexb_swizzle::guess_swizzle_patterns::<u64, _>(
                    swizzled_file,
                    deswizzled_file,
                    width,
                    height,
                    &format,
                ),
                ImageFormat::Bc3 | ImageFormat::Bc7 | ImageFormat::RgbaF32 => {
                    nutexb_swizzle::guess_swizzle_patterns::<u128, _>(
                        swizzled_file,
                        deswizzled_file,
                        width,
                        height,
                        &format,
                    )
                }
            };
            eprintln!("Calculate swizzle: {:?}", start.elapsed());
        }
        ("swizzle", Some(sub_m)) => {
            let width: usize = sub_m.value_of("width").unwrap().parse().unwrap();
            let height: usize = sub_m.value_of("height").unwrap().parse().unwrap();
            let input = sub_m.value_of("input").unwrap();
            let output = sub_m.value_of("output").unwrap();
            let format_text = sub_m.value_of("format").unwrap();
            let format = nutexb_swizzle::try_get_image_format(format_text).unwrap();

            swizzle(input, output, width, height, &format);
        }
        ("deswizzle", Some(sub_m)) => {
            let width: usize = sub_m.value_of("width").unwrap().parse().unwrap();
            let height: usize = sub_m.value_of("height").unwrap().parse().unwrap();
            let input = sub_m.value_of("input").unwrap();
            let output = sub_m.value_of("output").unwrap();
            let format_text = sub_m.value_of("format").unwrap();
            let format = nutexb_swizzle::try_get_image_format(format_text).unwrap();

            deswizzle(input, output, width, height, &format);
        }
        _ => (),
    }
}
