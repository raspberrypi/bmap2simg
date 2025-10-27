// SPDX-License-Identifier: BSD-3-Clause
// SPDX-FileCopyrightText: 2025 Raspberry Pi Ltd

use crate::bmap::{Bmap, BmapRange};
use crate::image_reader::ImageReader;
use std::io::{self, Read};

/// Calculate the actual number of bytes in a bmap range, limited by image size
pub fn calculate_range_bytes(bmap_range: &BmapRange, bmap: &Bmap) -> usize {
    let range_start_byte = bmap_range.range.start * bmap.block_size;
    let range_end_byte = bmap_range.range.end * bmap.block_size;

    // The actual bytes in this range is limited by the image size
    let actual_end_byte = range_end_byte.min(bmap.image_size);

    actual_end_byte.saturating_sub(range_start_byte)
}

/// Read data from reader, returning actual bytes read (may be less than requested on EOF)
pub fn read_exact_or_premature_eof(reader: &mut ImageReader, buf: &mut [u8]) -> io::Result<usize> {
    let mut total_read = 0;
    while total_read < buf.len() {
        match reader.read(&mut buf[total_read..])? {
            0 => return Ok(total_read), // Return what we got - caller will check if it's enough
            n => total_read += n,
        }
    }
    Ok(total_read)
}
