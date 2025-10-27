// SPDX-License-Identifier: BSD-3-Clause
// SPDX-FileCopyrightText: 2025 Raspberry Pi Ltd

use std::fs::File;
use std::io::BufReader;
use std::io::{self, Read, Seek, SeekFrom};

pub enum ImageReader {
    File(BufReader<File>),
    Stdin(BufReader<io::Stdin>),
}

impl Read for ImageReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            ImageReader::File(reader) => reader.read(buf),
            ImageReader::Stdin(reader) => reader.read(buf),
        }
    }
}

impl ImageReader {
    pub fn from_path_or_stdin(path: Option<&str>) -> std::io::Result<Self> {
        match path {
            Some(path) => Ok(ImageReader::File(BufReader::new(std::fs::File::open(
                path,
            )?))),
            None => Ok(ImageReader::Stdin(BufReader::new(std::io::stdin()))),
        }
    }

    pub fn skip_blocks(
        &mut self,
        blocks_to_skip: usize,
        block_size: usize,
        block_buf: &mut [u8],
    ) -> io::Result<()> {
        match self {
            ImageReader::File(reader) => {
                // Use seek for files - much faster!
                let bytes_to_skip = (blocks_to_skip * block_size) as i64;
                reader.seek(SeekFrom::Current(bytes_to_skip))?;
                Ok(())
            }
            ImageReader::Stdin(reader) => {
                // Fall back to reading and discarding for stdin
                // We need to read block_size bytes at a time, not the full buffer
                let mut remaining_blocks = blocks_to_skip;
                while remaining_blocks > 0 {
                    let blocks_this_read = remaining_blocks.min(block_buf.len() / block_size);
                    let bytes_to_read = blocks_this_read * block_size;
                    reader.read_exact(&mut block_buf[..bytes_to_read])?;
                    remaining_blocks -= blocks_this_read;
                }
                Ok(())
            }
        }
    }
}
