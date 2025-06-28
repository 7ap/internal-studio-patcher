#[cfg(target_os = "windows")]
mod stagescan;

#[cfg(target_os = "macos")]
mod stagescan_mac;

use std::fs;
use std::path::PathBuf;

#[cfg(target_os = "macos")]
use std::process::Command;

use std::time::Instant;
use clap::Parser;
#[cfg(windows)]
use winreg::{enums::HKEY_CLASSES_ROOT, RegKey};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    input: Option<PathBuf>,
    output: Option<PathBuf>,
}

fn patch(input: Option<PathBuf>, output: &PathBuf) {
    let input_path = input.as_ref().unwrap();
    let input = fs::read(input_path).unwrap();
    #[cfg(target_os = "macos")]
    {
        let now = Instant::now();
        stagescan_mac::start(input, output);
        println!("Patched in {:?}. Codesigning", now.elapsed());
        let codesigning_now = Instant::now();
        let status = Command::new("codesign")
        .arg("--force")
        .arg("--sign")
        .arg("-")
        .arg(&output)
        .status()
        .expect("Codesign command failed, studio may crash/not work properly due to codesigning failing");
        if !status.success() {
            eprintln!(
            "Codesign command failed, studio may crash/not work properly due to codesigning failing",
        );
            std::process::exit(1);
        }
        println!("Codesigning finished in {:?}.", codesigning_now.elapsed());
    }

    #[cfg(not(target_os = "macos"))]
    {
        let now = Instant::now();
        stagescan::start(input, output);
        println!("Patched in {:?}.", now.elapsed());
    }
}

fn main() {
    let Cli { mut input, output } = Cli::parse();
    #[cfg(target_os = "macos")]
    {
        input = input.or_else(|| {
            Some(PathBuf::from(
                "/Applications/RobloxStudio.app/Contents/MacOS/RobloxStudio",
            ))
        });
    }
    #[cfg(target_os = "windows")]
    {
        let path: String = RegKey::predef(HKEY_CLASSES_ROOT)
            .open_subkey("roblox-studio")
            .unwrap()
            .open_subkey("DefaultIcon")
            .unwrap()
            .get_value("")
            .unwrap();

        input = input.or_else(|| Some(PathBuf::from(path)));
    }
    let output = if cfg!(debug_assertions) {
        #[cfg(target_os = "macos")]
        {
            output.unwrap_or_else(|| {
                input
                    .as_ref()
                    .unwrap()
                    .with_file_name("RobloxStudioBeta_INTERNAL")
            })
        }
        #[cfg(target_os = "windows")]
        {
            output.unwrap_or_else(|| {
                input
                    .as_ref()
                    .unwrap()
                    .with_file_name("RobloxStudioBeta_INTERNAL.exe")
            })
        }
    } else {
        output.unwrap_or_else(|| input.as_ref().unwrap().clone())
    };
    #[cfg(target_os = "macos")]
    {
        if Command::new("codesign").arg("--version").output().is_err() { // can codesign not exist in some cases? no idea, will keep here though
            eprintln!("Error: codesign not found. Please run `xcode-select --install` first.");
            std::process::exit(1);
        }
    }
    patch(input, &output);
}
