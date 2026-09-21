use bootloader::UefiBoot;
use std::{env, path::PathBuf};

fn main() {
    let kernel_path = env::var("CARGO_BIN_FILE_KERNEL_kernel").expect(
        "CARGO_BIN_FILE_KERNEL_kernel not set - ensure that kernel is declared as an artifact dependency in Cargo.toml",
    );
    let kernel = PathBuf::from(kernel_path);
    let target_dir = PathBuf::from(env::var("CARGO_TARGET_DIR").unwrap_or("target".into()));
    let profile = env::var("PROFILE").expect("Profile is not set - this should always be provided by Cargo");
    let uefi_path = target_dir.join(&profile).join("uefi.img");

    println!("kernel path: {}", kernel.display());
    println!("UEFI path: {}", uefi_path.display());

    UefiBoot::new(&kernel).create_disk_image(&uefi_path).expect("Failed to create UEFI disk image");

    println!("cargo:rustc-env=UEFI_PATH={}", uefi_path.display());
}
