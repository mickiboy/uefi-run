extern crate clap;
extern crate fatfs;
extern crate tempfile;

use std::io::Write;
use std::process::Command;

fn main() {
    let matches = clap::App::new("uefi-run")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Richard Wiedenhöft <richard@wiedenhoeft.xyz>")
        .about("Runs UEFI executables in qemu.")
        .arg(
            clap::Arg::with_name("efi_exe")
                .value_name("FILE")
                .required(true)
                .help("EFI executable"),
        )
        .arg(
            clap::Arg::with_name("bios_path")
                .value_name("bios_path")
                .required(false)
                .help("BIOS image (default = /usr/share/OVMF/OVMF_CODE.fd)")
                .short("b")
                .long("bios"),
        )
        .arg(
            clap::Arg::with_name("qemu_path")
                .value_name("qemu_path")
                .required(false)
                .help("Path to qemu executable (default = /usr/bin/qemu-system-x86_64")
                .short("q")
                .long("qemu"),
        )
        .arg(
            clap::Arg::with_name("size")
                .value_name("size")
                .required(false)
                .help("Size of the image in MiB (default = 10)")
                .short("s")
                .long("size"),
        )
        .get_matches();

    // Parse options
    let efi_exe = matches.value_of("efi_exe").unwrap();
    let bios_path = matches
        .value_of("bios_path")
        .unwrap_or("/usr/share/OVMF/OVMF_CODE.fd");
    let qemu_path = matches
        .value_of("qemu_path")
        .unwrap_or("/usr/bin/qemu-system-x86_64");
    let size: u64 = matches
        .value_of("size")
        .map(|v| v.parse().expect("Failed to parse --size argument"))
        .unwrap_or(10);

    // Create temporary image file
    let image_file = tempfile::NamedTempFile::new().expect("Temporary image creation failed");
    // Truncate image to `size` MiB
    image_file
        .as_file()
        .set_len(size * 0x10_0000)
        .expect("Truncating image file failed");
    // Format file as FAT
    fatfs::format_volume(&image_file, fatfs::FormatVolumeOptions::new())
        .expect("Formatting image file failed");

    {
        let fs = fatfs::FileSystem::new(&image_file, fatfs::FsOptions::new()).unwrap();

        // Create folders
        let efi_dir = fs.root_dir().create_dir("EFI").unwrap();
        let boot_dir = efi_dir.create_dir("Boot").unwrap();

        // Create Bootx64.efi
        let efi_exe_contents = std::fs::read(efi_exe).unwrap();
        let mut run_efi = boot_dir.create_file("Bootx64.efi").unwrap();
        run_efi.truncate().unwrap();
        run_efi.write_all(&efi_exe_contents).unwrap();
    }

    // Run qemu and wait for it to terminate.
    let ecode = Command::new(qemu_path)
        .args(&[
            "-drive".into(),
            format!(
                "file={},index=0,media=disk,format=raw",
                image_file.path().display()
            ),
            "-bios".into(),
            format!("{}", bios_path),
            "-net".into(),
            "none".into(),
            "-serial".into(),
            "stdio".into(),
        ])
        .spawn()
        .expect("Failed to start qemu")
        .wait()
        .expect("Failed to wait on qemu");
    if !ecode.success() {
        println!("qemu execution failed");
    }
}
