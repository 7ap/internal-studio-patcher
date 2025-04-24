use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use clap::Parser;
use winreg::{enums::HKEY_CLASSES_ROOT, RegKey};

#[rustfmt::skip]
const SIGNATURE: &[u8] = &[
    0x00, 0x00, 0x80, 0xBF, 0x78, 0x01, 0x00, 0x00, 0x00, 0x74, 0x05, 0xE8
];

#[rustfmt::skip]
const PATCH: &[u8] = &[
    0x00, 0x00, 0x80, 0xBF, 0x78, 0x01, 0x00, 0x00, 0x00, 0x90, 0x90, 0xE8
];

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    input: Option<PathBuf>,
    output: Option<PathBuf>,
}

fn patch(input: &PathBuf, output: &PathBuf) {
    let mut binary = fs::read(input).expect("Could not read input file.");

    let offset = binary
        .windows(SIGNATURE.len())
        .position(|window| window == SIGNATURE)
        .expect("Could not find signature.");

    binary[offset..offset + PATCH.len()].copy_from_slice(PATCH);
    fs::write(output, binary).expect("Could not write output file.");
}

fn main() {
    let Cli { input, output } = Cli::parse();

    #[rustfmt::skip]
    let input = input.unwrap_or_else(|| {
        let path: String = RegKey::predef(HKEY_CLASSES_ROOT)
            .open_subkey("roblox-studio").unwrap()
            .open_subkey("DefaultIcon").unwrap()
            .get_value("").unwrap();

        PathBuf::from(path)
    });

    let output = if cfg!(debug_assertions) {
        output.unwrap_or_else(|| input.with_file_name("RobloxStudioBeta_INTERNAL.exe"))
    } else {
        output.unwrap_or_else(|| input.clone()) // TODO: Is convenience really worth it?
    };

    let now = Instant::now();
    patch(&input, &output);
    println!("Patched in {:?}.", now.elapsed());
}
