// SPDX-License-Identifier: BSD-3-Clause
// SPDX-FileCopyrightText: 2025 Raspberry Pi Ltd

use clap::Parser;
use std::error::Error;
use std::fs::File;
use std::io::{BufWriter, Write};

mod android_sparse;
mod bmap;
mod cli;
mod converter;
mod image_reader;
mod processor;
mod utils;

use crate::bmap::Bmap;
use crate::cli::Args;
use crate::converter::BmapSparseConverter;
use crate::image_reader::ImageReader;

fn parse_bmap_file(args: &Args) -> Result<Bmap, Box<dyn Error>> {
    let bmap_content = std::fs::read_to_string(&args.bmap)?;
    Ok(quick_xml::de::from_str::<Bmap>(&bmap_content)?)
}

fn create_image_reader(args: &Args) -> Result<ImageReader, Box<dyn Error>> {
    let path = args.input.as_ref().map(|p| p.to_str().unwrap());
    ImageReader::from_path_or_stdin(path).map_err(Into::into)
}

fn create_output_writer(args: &Args) -> Result<Box<dyn Write>, Box<dyn Error>> {
    match &args.output {
        Some(path) => {
            eprintln!("Creating sparse image: {}", path.display());
            Ok(Box::new(BufWriter::new(File::create(path)?)))
        }
        None => Ok(Box::new(BufWriter::new(std::io::stdout()))),
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let bmap = parse_bmap_file(&args)?;
    let image_reader = create_image_reader(&args)?;
    let output_writer = create_output_writer(&args)?;

    let mut converter = BmapSparseConverter::new(
        image_reader,
        output_writer,
        &bmap,
        None, // Use default buffer size
    );

    converter.convert(&bmap)
}
