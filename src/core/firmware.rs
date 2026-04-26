use std::path::Path;

use anyhow::{bail, Context};
use ihex::Record;
use probe_rs::flashing;

pub const FLASH_BASE: u32 = 0x0800_0000;

#[derive(Debug, Clone)]
pub struct FirmwareSegment {
    pub address: u32,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct FirmwareImage {
    pub kind_label: &'static str,
    pub segments: Vec<FirmwareSegment>,
    pub total_bytes: usize,
}

pub fn load_firmware_image(file_path: &str) -> anyhow::Result<FirmwareImage> {
    let lower = file_path.to_ascii_lowercase();
    if lower.ends_with(".bin") {
        load_bin_image(file_path)
    } else if lower.ends_with(".hex") {
        load_hex_image(file_path)
    } else {
        bail!("Unsupported file extension: {}", file_path)
    }
}

fn load_bin_image(file_path: &str) -> anyhow::Result<FirmwareImage> {
    let firmware = std::fs::read(file_path).context("Failed to read firmware file")?;
    if firmware.is_empty() {
        bail!("Firmware file is empty");
    }

    Ok(FirmwareImage {
        kind_label: "BIN",
        total_bytes: firmware.len(),
        segments: vec![FirmwareSegment {
            address: FLASH_BASE,
            data: firmware,
        }],
    })
}

fn load_hex_image(file_path: &str) -> anyhow::Result<FirmwareImage> {
    let text = std::fs::read_to_string(file_path).context("Failed to read HEX file")?;

    let mut upper_linear = 0u32;
    let mut upper_segment = 0u32;
    let mut use_linear = true;
    let mut chunks: Vec<(u32, Vec<u8>)> = Vec::new();

    for (line_idx, record) in ihex::Reader::new(&text).enumerate() {
        let record = record.with_context(|| format!("HEX parse error at line {}", line_idx + 1))?;
        match record {
            Record::Data { offset, value } => {
                if value.is_empty() {
                    continue;
                }

                let base = if use_linear {
                    upper_linear << 16
                } else {
                    upper_segment << 4
                };
                let address = base + u32::from(offset);

                if address < FLASH_BASE {
                    bail!(
                        "HEX contains address below flash region: 0x{:08X} (minimum 0x{:08X})",
                        address,
                        FLASH_BASE
                    );
                }

                match chunks.last_mut() {
                    Some((last_addr, last_data))
                        if *last_addr + last_data.len() as u32 == address =>
                    {
                        last_data.extend_from_slice(&value);
                    }
                    _ => chunks.push((address, value)),
                }
            }
            Record::ExtendedLinearAddress(addr) => {
                upper_linear = u32::from(addr);
                use_linear = true;
            }
            Record::ExtendedSegmentAddress(addr) => {
                upper_segment = u32::from(addr);
                use_linear = false;
            }
            Record::EndOfFile
            | Record::StartLinearAddress(_)
            | Record::StartSegmentAddress { .. } => {}
        }
    }

    if chunks.is_empty() {
        bail!("HEX file does not contain any writable flash data");
    }

    chunks.sort_by_key(|(address, _)| *address);
    for pair in chunks.windows(2) {
        let (addr_a, data_a) = (&pair[0].0, &pair[0].1);
        let (addr_b, _) = (&pair[1].0, &pair[1].1);
        let end_a = *addr_a + data_a.len() as u32;
        if *addr_b < end_a {
            bail!(
                "HEX contains overlapping ranges near 0x{:08X} and 0x{:08X}",
                addr_a,
                addr_b
            );
        }
    }

    let segments: Vec<FirmwareSegment> = chunks
        .into_iter()
        .map(|(address, data)| FirmwareSegment { address, data })
        .collect();
    let total_bytes = segments.iter().map(|segment| segment.data.len()).sum();

    Ok(FirmwareImage {
        kind_label: "HEX",
        segments,
        total_bytes,
    })
}

pub fn detect_file_kind(file_path: &str) -> Option<flashing::FormatKind> {
    let lower = file_path.to_ascii_lowercase();
    if lower.ends_with(".bin") {
        Some(flashing::FormatKind::Bin)
    } else if lower.ends_with(".hex") {
        Some(flashing::FormatKind::Hex)
    } else {
        None
    }
}

pub fn validate_firmware_path(file_path: &str) -> anyhow::Result<()> {
    if file_path.is_empty() {
        bail!("File path is empty");
    }
    if detect_file_kind(file_path).is_none() {
        bail!("Only .bin/.hex files are supported (got: {})", file_path);
    }
    if !Path::new(file_path).exists() {
        bail!("File not found: {}", file_path);
    }
    Ok(())
}

pub fn parse_address(input: &str) -> anyhow::Result<u64> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        bail!("BIN base address is empty");
    }

    if let Some(hex) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        u64::from_str_radix(hex, 16).context("BIN base address must be a valid hex number")
    } else {
        trimmed
            .parse::<u64>()
            .context("BIN base address must be a valid decimal or hex number")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- detect_file_kind ---

    #[test]
    fn detect_bin_lowercase() {
        assert!(matches!(
            detect_file_kind("firmware.bin"),
            Some(flashing::FormatKind::Bin)
        ));
    }

    #[test]
    fn detect_hex_lowercase() {
        assert!(matches!(
            detect_file_kind("firmware.hex"),
            Some(flashing::FormatKind::Hex)
        ));
    }

    #[test]
    fn detect_bin_uppercase() {
        assert!(matches!(
            detect_file_kind("FIRMWARE.BIN"),
            Some(flashing::FormatKind::Bin)
        ));
    }

    #[test]
    fn detect_hex_uppercase() {
        assert!(matches!(
            detect_file_kind("FIRMWARE.HEX"),
            Some(flashing::FormatKind::Hex)
        ));
    }

    #[test]
    fn detect_unknown_extension() {
        assert!(detect_file_kind("firmware.elf").is_none());
    }

    #[test]
    fn detect_no_extension() {
        assert!(detect_file_kind("firmware").is_none());
    }

    // --- validate_firmware_path ---

    #[test]
    fn validate_empty_path() {
        assert!(validate_firmware_path("").is_err());
    }

    #[test]
    fn validate_unsupported_extension() {
        let err = validate_firmware_path("build/output.elf").unwrap_err();
        assert!(err.to_string().contains("Only .bin/.hex"));
    }

    #[test]
    fn validate_file_not_found() {
        let err = validate_firmware_path("/nonexistent/path/fw.bin").unwrap_err();
        assert!(err.to_string().contains("File not found"));
    }

    // --- parse_address ---

    #[test]
    fn parse_hex_with_0x_prefix() {
        assert_eq!(parse_address("0x08000000").unwrap(), 0x0800_0000);
    }

    #[test]
    fn parse_hex_with_0x_upper_prefix() {
        assert_eq!(parse_address("0X08000000").unwrap(), 0x0800_0000);
    }

    #[test]
    fn parse_decimal() {
        assert_eq!(parse_address("134217728").unwrap(), 134_217_728);
    }

    #[test]
    fn parse_address_with_whitespace() {
        assert_eq!(parse_address("  0x08000000  ").unwrap(), 0x0800_0000);
    }

    #[test]
    fn parse_empty_address() {
        assert!(parse_address("").is_err());
    }

    #[test]
    fn parse_only_whitespace() {
        assert!(parse_address("   ").is_err());
    }

    #[test]
    fn parse_invalid_hex_digits() {
        assert!(parse_address("0xGGGG").is_err());
    }

    #[test]
    fn parse_invalid_decimal() {
        assert!(parse_address("not_a_number").is_err());
    }
}

pub fn format_for_download(
    file_path: &str,
    base_address: Option<u64>,
) -> anyhow::Result<flashing::Format> {
    match detect_file_kind(file_path) {
        Some(flashing::FormatKind::Bin) => Ok(flashing::Format::Bin(flashing::BinOptions {
            base_address,
            skip: 0,
        })),
        Some(flashing::FormatKind::Hex) => Ok(flashing::Format::Hex),
        _ => bail!("Only .bin/.hex files are supported (got: {})", file_path),
    }
}
