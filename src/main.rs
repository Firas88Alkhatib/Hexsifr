use std::{fs, path::PathBuf, process::Command};

use ovmf_prebuilt::{Arch, FileType, Prebuilt, Source};

fn main() {
    let prebuilt = Prebuilt::fetch(Source::LATEST, "target/ovmf").expect("Failed to fetch latest OVMF");
    let code = prebuilt.get_file(Arch::X64, FileType::Code);
    let vars = prebuilt.get_file(Arch::X64, FileType::Vars);

    let writable_vars = PathBuf::from("target/OVMF_VARS.fd");

    fs::copy(&vars, &writable_vars).expect("Failed to copy OVMF_VARS.fd");

    let uefi_path = env!("UEFI_PATH");

    Command::new("qemu-system-x86_64")
        .args(["-machine", "q35"])
        .args(["-accel", "kvm"])
        .args(["-cpu", "host"])
        .args(["-m", "4G"])
        .args(["-smp", "4"])
        .args(["-serial", "chardev:console"])
        .args(["-chardev", "stdio,id=console,mux=on"])
        .args(["-device", "isa-debugcon,iobase=0xe9,chardev=console"])
        .args(["-device", "virtio-keyboard-pci"])
        .args(["-device", "virtio-mouse-pci"])
        .args(["-device", "isa-debug-exit,iobase=0xf4,iosize=0x04"])
        .args(["-drive", format!("if=pflash,format=raw,readonly=on,file={}", code.display()).as_str()])
        .args(["-drive", format!("if=pflash,format=raw,file={}", writable_vars.display()).as_str()])
        .args(["-drive", format!("format=raw,file={}", uefi_path).as_str()])
        .status()
        .expect("Failed to run QEMU");
}
