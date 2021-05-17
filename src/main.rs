use clap::{App, AppSettings, Arg, SubCommand};
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
        .possible_values(&["bc1", "bc3", "bc7", "rgba"])
        .case_insensitive(true);

    let block_count_arg = Arg::with_name("blockcount")
        .long("blockcount")
        .help("The number of blocks to write or number of pixels for uncompressed data")
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
                .arg(&block_count_arg)
                .arg(
                    Arg::with_name("output")
                        .short("o")
                        .long("output")
                        .help("The output file for the image data")
                        .required(true)
                        .takes_value(true),
                ),
        )
        .get_matches();

    match matches.subcommand() {
        ("write_addresses", Some(sub_m)) => {
            let output = Path::new(sub_m.value_of("output").unwrap());
            let width: usize = sub_m.value_of("width").unwrap().parse().unwrap();
            let height: usize = sub_m.value_of("height").unwrap().parse().unwrap();
            let pixel_count = width * height;
            let pixels_per_tile = 4 * 4;

            let block_count: usize = match sub_m.value_of("blockcount") {
                Some(v) => v.parse().unwrap(),
                None => pixel_count / pixels_per_tile
            };

            let mut writer = std::fs::File::create(output).unwrap();

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
                    sub_m.value_of("format").unwrap(),
                    block_count,
                );
            } else {
                // TODO: RGBA
                match sub_m.value_of("format").unwrap() {
                    "bc1" => {
                        nutexb_swizzle::write_bc1_lut(&mut writer, block_count)
                    }
                    "bc3" => {
                        nutexb_swizzle::write_bc3_lut(&mut writer, block_count)
                    }
                    "bc7" => {
                        nutexb_swizzle::write_bc7_lut(&mut writer, block_count)
                    }
                    _ => (),
                }
            };
        }
        ("calculate_swizzle", Some(sub_m)) => {
            let width: usize = sub_m.value_of("width").unwrap().parse().unwrap();
            let height: usize = sub_m.value_of("height").unwrap().parse().unwrap();
            let swizzled_file = sub_m.value_of("swizzled").unwrap();
            let deswizzled_file = sub_m.value_of("deswizzled").unwrap();

            // TODO: RGBA
            match sub_m.value_of("format").unwrap() {
                "bc1" => nutexb_swizzle::calculate_swizzle_patterns::<u64, _>(
                    swizzled_file,
                    deswizzled_file,
                    width,
                    height,
                ),
                "bc3" => nutexb_swizzle::calculate_swizzle_patterns::<u128, _>(
                    swizzled_file,
                    deswizzled_file,
                    width,
                    height,
                ),
                "bc7" => nutexb_swizzle::calculate_swizzle_patterns::<u128, _>(
                    swizzled_file,
                    deswizzled_file,
                    width,
                    height,
                ),
                _ => unreachable!(),
            }
        }
        _ => (),
    }
}
