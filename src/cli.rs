// SPDX-License-Identifier: BSD-3-Clause
// SPDX-FileCopyrightText: 2025 Raspberry Pi Ltd

use clap::Parser;
use std::path::PathBuf;

/// Converts disk image files with block map (bmap) information to Android sparse images
///
/// Reads from stdin and writes to stdout by default, but input/output files can be specified.
#[derive(Parser)]
#[command(name = "bmap2simg")]
#[command(version = "0.1.0")]
pub struct Args {
    /// Block map file (.bmap)
    pub bmap: PathBuf,

    /// Input image file (default: stdin)
    #[arg(short = 'i', long = "input-img", value_name = "FILE")]
    pub input: Option<PathBuf>,

    /// Output sparse image file (default: stdout)
    #[arg(short = 'o', long = "output-simg", value_name = "FILE")]
    pub output: Option<PathBuf>,
}
