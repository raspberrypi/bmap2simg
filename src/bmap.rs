// SPDX-License-Identifier: BSD-3-Clause
// SPDX-FileCopyrightText: 2025 Raspberry Pi Ltd

use serde::{Deserialize, Deserializer};

#[derive(Deserialize, Debug)]
pub struct Bmap {
    #[serde(rename = "@version")]
    pub _version: String,
    #[serde(rename = "ImageSize")]
    pub image_size: usize,
    #[serde(rename = "BlockSize", deserialize_with = "deserialize_block_size")]
    pub block_size: usize,
    #[serde(rename = "BlocksCount")]
    pub blocks_count: usize,
    #[serde(rename = "MappedBlocksCount")]
    pub _mapped_blocks_count: usize,
    #[serde(
        rename = "ChecksumType",
        deserialize_with = "deserialize_checksum_type"
    )]
    pub _checksum_type: String,
    #[serde(rename = "BmapFileChecksum")]
    pub _bmap_file_checksum: String,
    #[serde(rename = "BlockMap", deserialize_with = "deserialize_block_map")]
    pub block_map: Vec<BmapRange>,
}

fn deserialize_block_size<'de, D>(deserializer: D) -> Result<usize, D::Error>
where
    D: Deserializer<'de>,
{
    let block_size = usize::deserialize(deserializer)?;
    if block_size != 4096 {
        return Err(serde::de::Error::custom(format!(
            "Unsupported block size: {block_size}. This program only supports block size 4096"
        )));
    }
    Ok(block_size)
}

fn deserialize_checksum_type<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let checksum_type = String::deserialize(deserializer)?;
    if checksum_type != "sha256" {
        return Err(serde::de::Error::custom(format!(
            "Unsupported checksum type: '{checksum_type}'. This program only supports 'sha256'"
        )));
    }
    Ok(checksum_type)
}

fn deserialize_block_map<'de, D>(deserializer: D) -> Result<Vec<BmapRange>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    struct BlockMapWrapper {
        #[serde(rename = "Range")]
        ranges: Vec<BmapRange>,
    }

    let wrapper = BlockMapWrapper::deserialize(deserializer)?;
    Ok(wrapper.ranges)
}

fn deserialize_sha256_hex<'de, D>(deserializer: D) -> Result<[u8; 32], D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    let s = s.trim();

    // Check if the hex string has the correct length (64 characters for 32 bytes)
    if s.len() != 64 {
        return Err(serde::de::Error::custom(format!(
            "Invalid SHA256 hex length: expected 64 characters, got {}",
            s.len()
        )));
    }

    // Parse hex string to bytes
    let mut bytes = [0u8; 32];
    for (i, chunk) in s.as_bytes().chunks(2).enumerate() {
        let hex_str = std::str::from_utf8(chunk)
            .map_err(|_| serde::de::Error::custom("Invalid UTF-8 in hex string"))?;
        bytes[i] = u8::from_str_radix(hex_str, 16).map_err(|_| {
            serde::de::Error::custom(format!("Invalid hex character in: {hex_str}"))
        })?;
    }

    Ok(bytes)
}

fn deserialize_bmap_range<'de, D>(deserializer: D) -> Result<std::ops::Range<usize>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    let s = s.trim();

    if let Some(dash_pos) = s.find('-') {
        // Range format: "10-22"
        let start_str = &s[..dash_pos];
        let end_str = &s[dash_pos + 1..];

        let start = start_str
            .parse::<usize>()
            .map_err(|_| serde::de::Error::custom(format!("Invalid start number: {start_str}")))?;
        let end = end_str
            .parse::<usize>()
            .map_err(|_| serde::de::Error::custom(format!("Invalid end number: {end_str}")))?;

        // Convert to inclusive range (end + 1 for exclusive end)
        Ok(start..end + 1)
    } else {
        // Single number format: "42"
        let num = s
            .parse::<usize>()
            .map_err(|_| serde::de::Error::custom(format!("Invalid number: {s}")))?;

        // Single number becomes a range of one element
        Ok(num..num + 1)
    }
}

#[derive(Deserialize, Debug)]
pub struct BmapRange {
    #[serde(rename = "$text", deserialize_with = "deserialize_bmap_range")]
    pub range: std::ops::Range<usize>,
    #[serde(rename = "@chksum", deserialize_with = "deserialize_sha256_hex")]
    pub chksum: [u8; 32],
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::de::IntoDeserializer;
    use serde::de::value::{Error as ValueError, StrDeserializer};

    #[test]
    fn test_deserialize_bmap_range_single_number() {
        let deserializer: StrDeserializer<ValueError> = "42".into_deserializer();
        let result = deserialize_bmap_range(deserializer).unwrap();
        assert_eq!(result, 42..43);
    }

    #[test]
    fn test_deserialize_bmap_range_with_whitespace() {
        let deserializer: StrDeserializer<ValueError> = "  42  ".into_deserializer();
        let result = deserialize_bmap_range(deserializer).unwrap();
        assert_eq!(result, 42..43);
    }

    #[test]
    fn test_deserialize_bmap_range_range_format() {
        let deserializer: StrDeserializer<ValueError> = "10-22".into_deserializer();
        let result = deserialize_bmap_range(deserializer).unwrap();
        assert_eq!(result, 10..23); // 22 + 1 for exclusive end
    }

    #[test]
    fn test_deserialize_bmap_range_range_with_whitespace() {
        let deserializer: StrDeserializer<ValueError> = "  10-22  ".into_deserializer();
        let result = deserialize_bmap_range(deserializer).unwrap();
        assert_eq!(result, 10..23);
    }

    #[test]
    fn test_deserialize_bmap_range_zero_values() {
        let deserializer: StrDeserializer<ValueError> = "0-2".into_deserializer();
        let result = deserialize_bmap_range(deserializer).unwrap();
        assert_eq!(result, 0..3);
    }

    #[test]
    fn test_deserialize_bmap_range_single_zero() {
        let deserializer: StrDeserializer<ValueError> = "0".into_deserializer();
        let result = deserialize_bmap_range(deserializer).unwrap();
        assert_eq!(result, 0..1);
    }

    #[test]
    fn test_deserialize_bmap_range_large_numbers() {
        let deserializer: StrDeserializer<ValueError> = "2048-27263".into_deserializer();
        let result = deserialize_bmap_range(deserializer).unwrap();
        assert_eq!(result, 2048..27264);
    }

    #[test]
    fn test_deserialize_bmap_range_same_start_end() {
        let deserializer: StrDeserializer<ValueError> = "100-100".into_deserializer();
        let result = deserialize_bmap_range(deserializer).unwrap();
        assert_eq!(result, 100..101);
    }

    #[test]
    fn test_deserialize_bmap_range_invalid_single_number() {
        let deserializer: StrDeserializer<ValueError> = "abc".into_deserializer();
        let result = deserialize_bmap_range(deserializer);
        assert!(result.is_err());
    }

    #[test]
    fn test_deserialize_bmap_range_invalid_range_start() {
        let deserializer: StrDeserializer<ValueError> = "abc-123".into_deserializer();
        let result = deserialize_bmap_range(deserializer);
        assert!(result.is_err());
    }

    #[test]
    fn test_deserialize_bmap_range_invalid_range_end() {
        let deserializer: StrDeserializer<ValueError> = "123-abc".into_deserializer();
        let result = deserialize_bmap_range(deserializer);
        assert!(result.is_err());
    }

    #[test]
    fn test_deserialize_bmap_range_empty_string() {
        let deserializer: StrDeserializer<ValueError> = "".into_deserializer();
        let result = deserialize_bmap_range(deserializer);
        assert!(result.is_err());
    }

    #[test]
    fn test_deserialize_bmap_range_multiple_dashes() {
        let deserializer: StrDeserializer<ValueError> = "10-20-30".into_deserializer();
        let result = deserialize_bmap_range(deserializer);
        // This should parse as "10" to "20-30", which should fail on the end part
        assert!(result.is_err());
    }

    #[test]
    fn test_deserialize_bmap_range_negative_numbers() {
        let deserializer: StrDeserializer<ValueError> = "-5".into_deserializer();
        let result = deserialize_bmap_range(deserializer);
        // This should fail as usize can't be negative
        assert!(result.is_err());
    }

    #[test]
    fn test_range_iteration() {
        // Test that the ranges work correctly for iteration
        let deserializer: StrDeserializer<ValueError> = "5-7".into_deserializer();
        let result = deserialize_bmap_range(deserializer).unwrap();
        let collected: Vec<usize> = result.collect();
        assert_eq!(collected, vec![5, 6, 7]);
    }

    #[test]
    fn test_single_number_iteration() {
        // Test that single number ranges work correctly for iteration
        let deserializer: StrDeserializer<ValueError> = "42".into_deserializer();
        let result = deserialize_bmap_range(deserializer).unwrap();
        let collected: Vec<usize> = result.collect();
        assert_eq!(collected, vec![42]);
    }

    #[test]
    fn test_deserialize_sha256_hex_valid() {
        let hex_str = "dc60c4950048f0283917518291fd2389bc9f3687824ece36d068ef4106eba826";
        let deserializer: StrDeserializer<ValueError> = hex_str.into_deserializer();
        let result = deserialize_sha256_hex(deserializer).unwrap();

        // Verify the first few bytes
        assert_eq!(result[0], 0xdc);
        assert_eq!(result[1], 0x60);
        assert_eq!(result[2], 0xc4);
        assert_eq!(result[3], 0x95);
        assert_eq!(result.len(), 32);
    }

    #[test]
    fn test_deserialize_sha256_hex_with_whitespace() {
        let hex_str = "  dc60c4950048f0283917518291fd2389bc9f3687824ece36d068ef4106eba826  ";
        let deserializer: StrDeserializer<ValueError> = hex_str.into_deserializer();
        let result = deserialize_sha256_hex(deserializer).unwrap();

        assert_eq!(result[0], 0xdc);
        assert_eq!(result.len(), 32);
    }

    #[test]
    fn test_deserialize_sha256_hex_invalid_length() {
        let hex_str = "dc60c4950048f0283917518291fd2389bc9f3687824ece36d068ef4106eba8"; // 63 chars
        let deserializer: StrDeserializer<ValueError> = hex_str.into_deserializer();
        let result = deserialize_sha256_hex(deserializer);
        assert!(result.is_err());
    }

    #[test]
    fn test_deserialize_sha256_hex_invalid_character() {
        let hex_str = "gc60c4950048f0283917518291fd2389bc9f3687824ece36d068ef4106eba826"; // 'g' is invalid
        let deserializer: StrDeserializer<ValueError> = hex_str.into_deserializer();
        let result = deserialize_sha256_hex(deserializer);
        assert!(result.is_err());
    }

    #[test]
    fn test_deserialize_sha256_hex_all_zeros() {
        let hex_str = "0000000000000000000000000000000000000000000000000000000000000000";
        let deserializer: StrDeserializer<ValueError> = hex_str.into_deserializer();
        let result = deserialize_sha256_hex(deserializer).unwrap();

        assert_eq!(result, [0u8; 32]);
    }

    #[test]
    fn test_deserialize_sha256_hex_all_ff() {
        let hex_str = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
        let deserializer: StrDeserializer<ValueError> = hex_str.into_deserializer();
        let result = deserialize_sha256_hex(deserializer).unwrap();

        assert_eq!(result, [0xffu8; 32]);
    }

    #[test]
    fn test_deserialize_block_size_valid() {
        use serde::de::value::UsizeDeserializer;
        let deserializer: UsizeDeserializer<ValueError> = 4096usize.into_deserializer();
        let result = deserialize_block_size(deserializer).unwrap();
        assert_eq!(result, 4096);
    }

    #[test]
    fn test_deserialize_block_size_invalid() {
        use serde::de::value::UsizeDeserializer;
        let deserializer: UsizeDeserializer<ValueError> = 512usize.into_deserializer();
        let result = deserialize_block_size(deserializer);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Unsupported block size: 512")
        );
    }

    #[test]
    fn test_deserialize_block_size_invalid_large() {
        use serde::de::value::UsizeDeserializer;
        let deserializer: UsizeDeserializer<ValueError> = 8192usize.into_deserializer();
        let result = deserialize_block_size(deserializer);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Unsupported block size: 8192")
        );
    }

    #[test]
    fn test_deserialize_checksum_type_valid() {
        let deserializer: StrDeserializer<ValueError> = "sha256".into_deserializer();
        let result = deserialize_checksum_type(deserializer).unwrap();
        assert_eq!(result, "sha256");
    }

    #[test]
    fn test_deserialize_checksum_type_invalid_md5() {
        let deserializer: StrDeserializer<ValueError> = "md5".into_deserializer();
        let result = deserialize_checksum_type(deserializer);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Unsupported checksum type: 'md5'")
        );
    }

    #[test]
    fn test_deserialize_checksum_type_invalid_sha1() {
        let deserializer: StrDeserializer<ValueError> = "sha1".into_deserializer();
        let result = deserialize_checksum_type(deserializer);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Unsupported checksum type: 'sha1'")
        );
    }

    #[test]
    fn test_deserialize_checksum_type_invalid_case() {
        let deserializer: StrDeserializer<ValueError> = "SHA256".into_deserializer();
        let result = deserialize_checksum_type(deserializer);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Unsupported checksum type: 'SHA256'")
        );
    }
}
