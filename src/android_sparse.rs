// SPDX-License-Identifier: BSD-3-Clause
// SPDX-FileCopyrightText: 2025 Raspberry Pi Ltd

//! Android Sparse Image Format Implementation
//!
//! This module implements the Android sparse image format as defined in the AOSP.
//!
//! ## Endianness Assumption
//!
//! This implementation assumes the target system is little-endian (x86/x64/ARM64).
//! The Android sparse format specification requires little-endian byte order,
//! and all modern Android-compatible systems are little-endian.

use std::io::{self, Write};
use zerocopy::AsBytes;

/// Magic number for Android sparse format: 0xed26ff3a
pub const SPARSE_HEADER_MAGIC: u32 = 0xed26ff3a;

/// Major version (0x1)
pub const MAJOR_VERSION: u16 = 0x1;

/// Minor version (0x0)
pub const MINOR_VERSION: u16 = 0x0;

/// Size of sparse header in bytes
pub const SPARSE_HEADER_SIZE: u16 = 28;

/// Size of chunk header in bytes
pub const CHUNK_HEADER_SIZE: u16 = 12;

/// Chunk types
#[derive(Debug, Clone, Copy, PartialEq, Eq, AsBytes)]
#[repr(u16)]
pub enum ChunkType {
    /// Raw data chunk (0xCAC1)
    Raw = 0xCAC1,
    /// Don't care chunk - can be anything (0xCAC3)
    DontCare = 0xCAC3,
}

/// Sparse header structure (28 bytes)
#[derive(Debug, Clone, AsBytes)]
#[repr(C)]
pub struct SparseHeader {
    /// Magic number (0xed26ff3a)
    pub magic: u32,
    /// Major version (0x1)
    pub major_version: u16,
    /// Minor version (0x0)
    pub minor_version: u16,
    /// File header size in bytes (28)
    pub file_hdr_sz: u16,
    /// Chunk header size in bytes (12)
    pub chunk_hdr_sz: u16,
    /// Block size in bytes, must be multiple of 4 (typically 4096)
    pub blk_sz: u32,
    /// Total blocks in the non-sparse output image
    pub total_blks: u32,
    /// Total chunks in the sparse input image
    pub total_chunks: u32,
    /// CRC32 checksum of the original data
    pub image_checksum: u32,
}

impl SparseHeader {
    /// Create a new sparse header with default values
    pub fn new(blk_sz: u32, total_blks: u32, total_chunks: u32, image_checksum: u32) -> Self {
        Self {
            magic: SPARSE_HEADER_MAGIC,
            major_version: MAJOR_VERSION,
            minor_version: MINOR_VERSION,
            file_hdr_sz: SPARSE_HEADER_SIZE,
            chunk_hdr_sz: CHUNK_HEADER_SIZE,
            blk_sz,
            total_blks,
            total_chunks,
            image_checksum,
        }
    }

    /// Write the header directly to a writer
    ///
    /// Note: Assumes the target system is little-endian (x86/x64/ARM64).
    /// The Android sparse format specifies little-endian byte order.
    pub fn write_to<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(self.as_bytes())
    }
}

/// Chunk header structure (12 bytes)
#[derive(Debug, Clone, AsBytes)]
#[repr(C)]
pub struct ChunkHeader {
    /// Chunk type (RAW, DONT_CARE)
    pub chunk_type: ChunkType,
    /// Reserved field (should be 0)
    pub reserved1: u16,
    /// Size in blocks in output image
    pub chunk_sz: u32,
    /// Total size in bytes of chunk including header and data
    pub total_sz: u32,
}

impl ChunkHeader {
    /// Create a new chunk header
    pub fn new(chunk_type: ChunkType, chunk_sz: u32, total_sz: u32) -> Self {
        Self {
            chunk_type,
            reserved1: 0,
            chunk_sz,
            total_sz,
        }
    }

    /// Write the chunk header directly to a writer
    ///
    /// Note: Assumes the target system is little-endian (x86/x64/ARM64).
    /// The Android sparse format specifies little-endian byte order.
    pub fn write_to<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(self.as_bytes())
    }
}

/// A complete chunk with header and data
#[derive(Debug, Clone)]
pub struct Chunk {
    pub header: ChunkHeader,
}

impl Chunk {
    /// Create a new don't care chunk
    pub fn new_dont_care(chunk_sz: u32) -> Self {
        let total_sz = CHUNK_HEADER_SIZE as u32;
        let header = ChunkHeader::new(ChunkType::DontCare, chunk_sz, total_sz);
        Self { header }
    }

    /// Write the complete chunk (header only for don't care) to a writer
    pub fn write_to<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        self.header.write_to(writer)?;
        // Don't care chunks have no data to write
        Ok(())
    }
}
