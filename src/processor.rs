// SPDX-License-Identifier: BSD-3-Clause
// SPDX-FileCopyrightText: 2025 Raspberry Pi Ltd

use crate::android_sparse;
use crate::bmap::Bmap;
use crate::image_reader::ImageReader;
use crate::utils::{calculate_range_bytes, read_exact_or_premature_eof};
use std::io::Write;

pub struct ProcessingConfig {
    pub buffer_size: usize, // e.g., 2 * 1024 * 1024 (2MiB)
    _block_size: usize,
}

impl ProcessingConfig {
    pub fn new(block_size: usize, buffer_size_hint: Option<usize>) -> Self {
        let default_buffer_size = 2 * 1024 * 1024; // 2MiB
        let buffer_size = buffer_size_hint.unwrap_or(default_buffer_size);

        // Ensure buffer is multiple of block_size and at least one block
        let buffer_size = ((buffer_size / block_size).max(1)) * block_size;

        Self {
            buffer_size,
            _block_size: block_size,
        }
    }
}

#[rustfmt::skip]
/*
     ┌──────────────────┐
     │      START       │
     └──────────────────┘
       │
       │
       ▼
     ╭──────────────────────────────╮     ╭──────────────────╮
  ┌▶ │     DeterminingNextRange     │ ──▶ │ WritingFinalSkip │
  │  ╰──────────────────────────────╯     ╰──────────────────╯
  │    │                   │      │         │
  │    │                   │      │         │
  │    ▼                   │      │         ▼
  │  ╭──────────────────╮  │      │       ┌──────────────────┐
  │  │ SkippingToRange  │  │      └─────▶ │     Complete     │
  │  ╰──────────────────╯  │              └──────────────────┘
  │    │                   │
  │    │                   │
  │    ▼                   │
  │  ╭──────────────────╮  │
  │  │  StartingRange   │ ◀┘
  │  ╰──────────────────╯
  │    │
  │    │
  │    ▼
  │  ╭──────────────────╮
  │  │                  │ ───┐
  │  │ ReadingRangeData │    │
  │  │                  │ ◀──┘
  │  ╰──────────────────╯
  │    │
  │    │
  │    ▼
  │  ╭──────────────────╮
  └─ │  FinishingRange  │
     ╰──────────────────╯
*/
enum ProcessingState<'a> {
    /// Need to skip blocks before processing next range
    SkippingToRange {
        target_range: &'a crate::bmap::BmapRange,
        current_block_pos: usize,
        chunks_written: u32,
    },
    /// Starting to process a new range (write sparse chunk header)
    StartingRange {
        current_range: &'a crate::bmap::BmapRange,
        chunks_written: u32,
    },
    /// Reading data within a range. This state will be visited several times
    /// for each range as only one buffers worth of data is read at a time.
    ReadingRangeData {
        current_range: &'a crate::bmap::BmapRange,
        bytes_processed_in_range: usize,
        chunks_written: u32,
        sha: openssl::sha::Sha256,
    },
    /// Finished reading a range, need to verify hash and transition
    FinishingRange {
        current_range: &'a crate::bmap::BmapRange,
        chunks_written: u32,
        sha: openssl::sha::Sha256,
    },
    /// Determine what to do next: skip to next range, start next range, or finish
    DeterminingNextRange {
        current_block_pos: usize,
        chunks_written: u32,
    },
    /// Need to write final skip chunk after all ranges
    WritingFinalSkip {
        chunks_written: u32,
        skip_blocks: usize,
    },
    /// Processing complete
    Complete { chunks_written: u32 },
}

impl<'a> ProcessingState<'a> {
    fn new() -> Self {
        // Start by determining what to do with the first range
        ProcessingState::DeterminingNextRange {
            current_block_pos: 0,
            chunks_written: 0,
        }
    }

    fn process_next(
        self,
        image_reader: &mut ImageReader,
        output_file: &mut Box<dyn Write>,
        bmap: &Bmap,
        config: &ProcessingConfig,
        buffer: &mut [u8],
        range_iter: &mut std::slice::Iter<'a, crate::bmap::BmapRange>,
    ) -> Result<ProcessingState<'a>, Box<dyn std::error::Error>> {
        match self {
            ProcessingState::SkippingToRange {
                target_range,
                current_block_pos,
                chunks_written,
            } => {
                let blocks_to_skip = target_range.range.start - current_block_pos;

                image_reader.skip_blocks(blocks_to_skip, bmap.block_size, buffer)?;

                // Write skip chunk
                let skip_chunk = android_sparse::Chunk::new_dont_care(blocks_to_skip as u32);
                skip_chunk.write_to(output_file)?;

                Ok(ProcessingState::StartingRange {
                    current_range: target_range,
                    chunks_written: chunks_written + 1,
                })
            }

            ProcessingState::StartingRange {
                current_range,
                chunks_written,
            } => {
                let total_bytes_in_range = calculate_range_bytes(current_range, bmap);

                // Write chunk header for this range
                let total_blocks_in_range =
                    (current_range.range.end - current_range.range.start) as u32;
                let chunk_header = android_sparse::ChunkHeader::new(
                    android_sparse::ChunkType::Raw,
                    total_blocks_in_range,
                    (android_sparse::CHUNK_HEADER_SIZE as u32) + total_bytes_in_range as u32,
                );
                chunk_header.write_to(output_file)?;

                // Transition to reading data
                Ok(ProcessingState::ReadingRangeData {
                    current_range,
                    bytes_processed_in_range: 0,
                    chunks_written: chunks_written + 1,
                    sha: openssl::sha::Sha256::new(),
                })
            }

            ProcessingState::ReadingRangeData {
                current_range,
                bytes_processed_in_range,
                chunks_written,
                mut sha,
            } => {
                let total_bytes_in_range = calculate_range_bytes(current_range, bmap);

                // Check if we've finished reading this range
                if bytes_processed_in_range >= total_bytes_in_range {
                    return Ok(ProcessingState::FinishingRange {
                        current_range,
                        chunks_written,
                        sha,
                    });
                }

                // Continue reading data
                let bytes_remaining_in_range = total_bytes_in_range - bytes_processed_in_range;
                let bytes_to_read = bytes_remaining_in_range.min(config.buffer_size);

                // Read the data
                let bytes_read =
                    read_exact_or_premature_eof(image_reader, &mut buffer[..bytes_to_read])?;

                if bytes_read < bytes_to_read {
                    return Err(Box::new(std::io::Error::other(format!(
                        "Premature EOF: expected {bytes_to_read} bytes, got {bytes_read} bytes in range {current_range:?}"
                    ))));
                }

                // Write data and update hash
                sha.update(&buffer[..bytes_read]);
                output_file.write_all(&buffer[..bytes_read])?;

                Ok(ProcessingState::ReadingRangeData {
                    current_range,
                    bytes_processed_in_range: bytes_processed_in_range + bytes_read,
                    chunks_written,
                    sha,
                })
            }

            ProcessingState::FinishingRange {
                current_range,
                chunks_written,
                sha,
            } => {
                // Verify hash
                let hash = sha.finish();
                if hash != current_range.chksum {
                    return Err(Box::new(std::io::Error::other("Hash mismatch")));
                }

                // Update block position to end of current range and determine next action
                let updated_block_pos = current_range.range.end;
                Ok(ProcessingState::DeterminingNextRange {
                    current_block_pos: updated_block_pos,
                    chunks_written,
                })
            }

            ProcessingState::DeterminingNextRange {
                current_block_pos,
                chunks_written,
            } => {
                // Try to get the next range from the iterator
                let Some(next_range) = range_iter.next() else {
                    // All ranges processed - check for final skip
                    let final_skip_blocks = bmap.blocks_count - current_block_pos;
                    return Ok(if final_skip_blocks > 0 {
                        ProcessingState::WritingFinalSkip {
                            chunks_written,
                            skip_blocks: final_skip_blocks,
                        }
                    } else {
                        ProcessingState::Complete { chunks_written }
                    });
                };

                // Determine if we need to skip to the next range or start immediately
                Ok(if current_block_pos < next_range.range.start {
                    ProcessingState::SkippingToRange {
                        target_range: next_range,
                        current_block_pos,
                        chunks_written,
                    }
                } else {
                    ProcessingState::StartingRange {
                        current_range: next_range,
                        chunks_written,
                    }
                })
            }

            ProcessingState::WritingFinalSkip {
                chunks_written,
                skip_blocks,
            } => {
                let skip_chunk = android_sparse::Chunk::new_dont_care(skip_blocks as u32);
                skip_chunk.write_to(output_file)?;

                Ok(ProcessingState::Complete {
                    chunks_written: chunks_written + 1,
                })
            }

            ProcessingState::Complete { chunks_written } => {
                Ok(ProcessingState::Complete { chunks_written })
            }
        }
    }

    fn is_complete(&self) -> bool {
        matches!(self, ProcessingState::Complete { .. })
    }

    fn chunks_written(&self) -> u32 {
        match self {
            ProcessingState::SkippingToRange { chunks_written, .. } => *chunks_written,
            ProcessingState::StartingRange { chunks_written, .. } => *chunks_written,
            ProcessingState::ReadingRangeData { chunks_written, .. } => *chunks_written,
            ProcessingState::FinishingRange { chunks_written, .. } => *chunks_written,
            ProcessingState::DeterminingNextRange { chunks_written, .. } => *chunks_written,
            ProcessingState::WritingFinalSkip { chunks_written, .. } => *chunks_written,
            ProcessingState::Complete { chunks_written } => *chunks_written,
        }
    }
}

/// Process a bmap file using the state machine
pub fn process_bmap_with_state_machine(
    image_reader: &mut ImageReader,
    output_file: &mut Box<dyn Write>,
    bmap: &Bmap,
    config: &ProcessingConfig,
) -> Result<u32, Box<dyn std::error::Error>> {
    let mut buffer = vec![0u8; config.buffer_size];
    let mut state = ProcessingState::new();
    let mut range_iter = bmap.block_map.iter();

    while !state.is_complete() {
        state = state.process_next(
            image_reader,
            output_file,
            bmap,
            config,
            &mut buffer,
            &mut range_iter,
        )?;
    }

    Ok(state.chunks_written())
}

/// Calculate the total number of chunks that will be written
pub fn calculate_total_chunks(bmap: &Bmap, _config: &ProcessingConfig) -> u32 {
    // 2n + 1 accounts for don't care chunks between each range. Any start and
    // end skip chunks are calculated separately.
    let base_chunks = 2 * bmap.block_map.len() - 1;

    let start_skip_chunks = bmap.block_map.first().map(|r| r.range.start).unwrap_or(0);
    let start_skip_count = if start_skip_chunks > 0 { 1 } else { 0 };

    let end_skip_chunks = bmap.blocks_count
        - bmap
            .block_map
            .last()
            .map(|r| r.range.end)
            .unwrap_or(bmap.blocks_count);
    let end_skip_count = if end_skip_chunks > 0 { 1 } else { 0 };

    (base_chunks + start_skip_count + end_skip_count) as u32
}
