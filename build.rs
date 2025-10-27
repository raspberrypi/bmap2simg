use clap::CommandFactory;
use clap_mangen::Man;
use clap_mangen::roff::{Roff, roman};
use std::env;
use std::fs;

include!("src/cli.rs");

fn main() -> std::io::Result<()> {
    // Use OUT_DIR for build artifacts, not source directory
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    let cmd = Args::command();
    let man = Man::new(cmd);

    let mut buffer: Vec<u8> = Default::default();

    // Render standard sections
    man.render_title(&mut buffer)?;
    man.render_name_section(&mut buffer)?;
    man.render_synopsis_section(&mut buffer)?;
    man.render_description_section(&mut buffer)?;
    man.render_options_section(&mut buffer)?;

    // Add custom EXAMPLES section
    let mut roff = Roff::default();
    roff.control("SH", ["EXAMPLES"]);
    roff.text([roman("Basic usage (stdin/stdout):")]);
    roff.text([roman("    bmap2simg file.bmap < input.img > output.simg")]);
    roff.text([roman("")]);
    roff.text([roman("With input file flag:")]);
    roff.text([roman("    bmap2simg file.bmap -i input.img > output.simg")]);
    roff.text([roman("")]);
    roff.text([roman("With output file flag:")]);
    roff.text([roman("    bmap2simg file.bmap -o output.simg < input.img")]);
    roff.text([roman("")]);
    roff.text([roman("With both flags:")]);
    roff.text([roman("    bmap2simg file.bmap -i input.img -o output.simg")]);
    roff.text([roman("")]);
    roff.text([roman("Pipeline with XZ compression:")]);
    roff.text([roman("    xzcat 2025-10-01-raspios-trixie-arm64.img.xz \\")]);
    roff.text([roman(
        "      | bmap2simg 2025-10-01-raspios-trixie-arm64.bmap \\",
    )]);
    roff.text([roman("      | xz -6 \\")]);
    roff.text([roman("        > 2025-10-01-raspios-trixie-arm64.simg.xz")]);
    roff.text([roman("")]);
    roff.text([roman("This example demonstrates efficient processing of compressed images: decompress the XZ-compressed")]);
    roff.text([roman("image on-the-fly, convert it to sparse format using the block map, then recompress the sparse")]);
    roff.text([roman("image. This avoids storing the full uncompressed image on disk while creating a compressed")]);
    roff.text([roman(
        "sparse image that preserves the original block structure.",
    )]);

    roff.to_writer(&mut buffer)?;

    fs::write(out_dir.join("bmap2simg.1"), buffer)?;

    println!("cargo:rerun-if-changed=src/cli.rs");
    println!("cargo:rerun-if-changed=src/main.rs");

    Ok(())
}
