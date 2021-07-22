use clap::{App, AppSettings, Arg, SubCommand};
use nutexb_swizzle::formats::ImageFormat;
use nutexb_swizzle::{deswizzle, swizzle};
use std::path::Path;
use std::str::FromStr;

fn main() {
    // TODO: Use a yaml to configure this?
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
                .about("Prints swizzle patterns for all provided mipmaps")
                .arg(&format_arg)
                .arg(&width_arg)
                .arg(&height_arg)
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
                ),
        )
        .subcommand(
            SubCommand::with_name("write_addresses")
                .about("Writes a set number of unique blocks compatible with the given format")
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
                .about("Swizzles the input using experimental swizzle functions")
                .arg(&format_arg)
                .arg(&width_arg)
                .arg(&height_arg)
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
                ),
        )
        .subcommand(
            SubCommand::with_name("deswizzle")
                .about("Deswizzles the input using experimental swizzle functions")
                .arg(&format_arg)
                .arg(&width_arg)
                .arg(&height_arg)
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
                ),
        )
        .subcommand(
            SubCommand::with_name("write_swizzle_lut")
                .about("Writes swizzled and deswizzled address pairs in CSV format")
                .arg(&format_arg)
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
                .arg(
                    Arg::with_name("output")
                        .short("o")
                        .long("output")
                        .help("The output CSV file")
                        .required(true)
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("normalize")
                        .long("normalize")
                        .help("Shift swizzled indices to start from 0")
                        .required(false)
                        .takes_value(false)
                ),
        )
        .subcommand(
            SubCommand::with_name("extract_mipmaps")
                .about("Extracts mipmaps to separate files")
                .arg(&format_arg)
                .arg(
                    Arg::with_name("input")
                        .short("i")
                        .long("input")
                        .help("The input image data in DDS format")
                        .required(true)
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("output")
                        .short("o")
                        .long("output")
                        .help("The base name of the outputs. Files will be named as <output>_<mip_number>.bin")
                        .required(true)
                        .takes_value(true),
                )
        )
        .get_matches();

    let start = std::time::Instant::now();
    match matches.subcommand() {
        ("write_addresses", Some(sub_m)) => {
            write_addresses(sub_m);
        }
        ("calculate_swizzle", Some(sub_m)) => {
            calculate_swizzle(sub_m);
        }
        ("swizzle", Some(sub_m)) => {
            let width: usize = sub_m.value_of("width").unwrap().parse().unwrap();
            let height: usize = sub_m.value_of("height").unwrap().parse().unwrap();
            warn_if_not_power_of_two(width, height);
            let input = sub_m.value_of("input").unwrap();
            let output = sub_m.value_of("output").unwrap();
            let format_text = sub_m.value_of("format").unwrap();
            let format = ImageFormat::from_str(format_text).unwrap();

            swizzle(input, output, width, height, &format);
        }
        ("deswizzle", Some(sub_m)) => {
            let width: usize = sub_m.value_of("width").unwrap().parse().unwrap();
            let height: usize = sub_m.value_of("height").unwrap().parse().unwrap();
            warn_if_not_power_of_two(width, height);
            let input = sub_m.value_of("input").unwrap();
            let output = sub_m.value_of("output").unwrap();
            let format_text = sub_m.value_of("format").unwrap();
            let format = ImageFormat::from_str(format_text).unwrap();

            deswizzle(input, output, width, height, &format);
        }
        ("write_swizzle_lut", Some(sub_m)) => {
            let swizzled_file = sub_m.value_of("swizzled").unwrap();
            let deswizzled_file = sub_m.value_of("deswizzled").unwrap();
            let output = sub_m.value_of("output").unwrap();
            let format_text = sub_m.value_of("format").unwrap();
            let format = ImageFormat::from_str(format_text).unwrap();
            let normalize_indices = sub_m.is_present("normalize");

            nutexb_swizzle::write_lut_csv(swizzled_file, deswizzled_file, output, &format, normalize_indices);
        }
        ("extract_mipmaps", Some(sub_m)) => {
            let input = sub_m.value_of("input").unwrap();
            let output = sub_m.value_of("output").unwrap();
            let format_text = sub_m.value_of("format").unwrap();
            let format = ImageFormat::from_str(format_text).unwrap();

            nutexb_swizzle::extract_mipmaps(input, output, &format);
        }
        _ => (),
    }
    eprintln!("Command executed in {:?}", start.elapsed());
}

fn calculate_swizzle(sub_m: &clap::ArgMatches) {
    let width: usize = sub_m.value_of("width").unwrap().parse().unwrap();
    let height: usize = sub_m.value_of("height").unwrap().parse().unwrap();
    warn_if_not_power_of_two(width, height);
    let swizzled_file = sub_m.value_of("swizzled").unwrap();
    let deswizzled_file = sub_m.value_of("deswizzled").unwrap();
    let format = ImageFormat::from_str(sub_m.value_of("format").unwrap()).unwrap();
    calculate_swizzle_with_format(format, swizzled_file, deswizzled_file, width, height);
}

fn warn_if_not_power_of_two(width: usize, height: usize) {
    if !width.is_power_of_two() || !height.is_power_of_two() {
        eprintln!("Warning: Dimensions {}x{} are not powers of two. The operation assumes dimensions are powers of two.", width, height);
    }
}

fn calculate_swizzle_with_format(
    format: ImageFormat,
    swizzled_file: &str,
    deswizzled_file: &str,
    width: usize,
    height: usize,
) {
    match format {
        ImageFormat::Rgba8 => nutexb_swizzle::print_swizzle_patterns::<u32, _>(
            swizzled_file,
            deswizzled_file,
            width,
            height,
            &format,
        ),
        ImageFormat::Bc1 => nutexb_swizzle::print_swizzle_patterns::<u64, _>(
            swizzled_file,
            deswizzled_file,
            width,
            height,
            &format,
        ),
        ImageFormat::Bc3 | ImageFormat::Bc7 | ImageFormat::RgbaF32 => {
            nutexb_swizzle::print_swizzle_patterns::<u128, _>(
                swizzled_file,
                deswizzled_file,
                width,
                height,
                &format,
            )
        }
    };
}

fn write_addresses(sub_m: &clap::ArgMatches) {
    let output = Path::new(sub_m.value_of("output").unwrap());
    let width: usize = sub_m.value_of("width").unwrap().parse().unwrap();
    let height: usize = sub_m.value_of("height").unwrap().parse().unwrap();
    let format = ImageFormat::from_str(sub_m.value_of("format").unwrap()).unwrap();
    let block_count: usize = match sub_m.value_of("imagesize") {
        Some(v) => {
            let image_size: usize = v.parse().unwrap();
            image_size / format.get_tile_size_in_bytes()
        }
        None => format.get_tile_count(width, height),
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
            ImageFormat::RgbaF32 => nutexb_swizzle::write_rgba_f32_lut(&mut writer, block_count),
            ImageFormat::Bc1 => nutexb_swizzle::write_bc1_lut(&mut writer, block_count),
            ImageFormat::Bc3 => nutexb_swizzle::write_bc3_lut(&mut writer, block_count),
            ImageFormat::Bc7 => nutexb_swizzle::write_bc7_lut(&mut writer, block_count),
        }
    };
}
