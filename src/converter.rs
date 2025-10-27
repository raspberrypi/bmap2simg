// SPDX-License-Identifier: BSD-3-Clause
// SPDX-FileCopyrightText: 2025 Raspberry Pi Ltd

use crate::android_sparse;
use crate::bmap::Bmap;
use crate::image_reader::ImageReader;
use crate::processor::{ProcessingConfig, calculate_total_chunks, process_bmap_with_state_machine};
use std::io::Write;

pub struct BmapSparseConverter {
    image_reader: ImageReader,
    output_file: Box<dyn Write>,
    config: ProcessingConfig,
}

impl BmapSparseConverter {
    pub fn new(
        image_reader: ImageReader,
        output_file: Box<dyn Write>,
        bmap: &Bmap,
        buffer_size_hint: Option<usize>,
    ) -> Self {
        let config = ProcessingConfig::new(bmap.block_size, buffer_size_hint);

        Self {
            image_reader,
            output_file,
            config,
        }
    }

    pub fn convert(&mut self, bmap: &Bmap) -> Result<(), Box<dyn std::error::Error>> {
        // Calculate and write sparse header
        let num_sparse_chunks = calculate_total_chunks(bmap, &self.config);
        let sparse_header = android_sparse::SparseHeader::new(
            bmap.block_size as u32,
            bmap.blocks_count as u32,
            num_sparse_chunks,
            0,
        );
        sparse_header.write_to(&mut self.output_file)?;

        // Process with state machine
        let chunks_written = process_bmap_with_state_machine(
            &mut self.image_reader,
            &mut self.output_file,
            bmap,
            &self.config,
        )?;

        // Verify chunk count
        if chunks_written != num_sparse_chunks {
            return Err(Box::new(std::io::Error::other(format!(
                "Chunk count mismatch: promised {num_sparse_chunks} chunks but wrote {chunks_written}"
            ))));
        }

        Ok(())
    }
}
