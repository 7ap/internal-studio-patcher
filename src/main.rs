mod stagescan;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use clap::Parser;
use winreg::{enums::HKEY_CLASSES_ROOT, RegKey};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    input: Option<PathBuf>,
    output: Option<PathBuf>,
}

fn patch(input: &PathBuf, output: &PathBuf) {
    let input = fs::read(input).unwrap();
    stagescan::start(input, output);
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
