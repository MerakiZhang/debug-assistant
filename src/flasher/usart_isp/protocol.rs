//! AN3155 STM32 USART bootloader protocol implementation.
//! All serial I/O uses the port opened by the caller (8E1 settings required).
use anyhow::{bail, Context};
use serialport::ClearBuffer;
use std::time::{Duration, Instant};

const ACK: u8 = 0x79;
const NACK: u8 = 0x1F;

/// Read exactly one byte with a deadline.
fn read_byte(port: &mut dyn serialport::SerialPort) -> anyhow::Result<u8> {
    let mut buf = [0u8; 1];
    port.read_exact(&mut buf)
        .context("Read timeout waiting for response")?;
    Ok(buf[0])
}

/// Expect an ACK byte; return error if NACK or anything else.
fn expect_ack(port: &mut dyn serialport::SerialPort) -> anyhow::Result<()> {
    match read_byte(port)? {
        ACK => Ok(()),
        NACK => bail!("Bootloader returned NACK"),
        b => bail!("Unexpected byte: 0x{:02X} (expected ACK 0x79)", b),
    }
}

/// Compute XOR checksum of a byte slice.
fn xor_checksum(data: &[u8]) -> u8 {
    data.iter().fold(0u8, |acc, &b| acc ^ b)
}

/// Step 1: Synchronize with bootloader. Send 0x7F and expect ACK.
/// Retries up to `retries` times with a short flush in between.
pub fn sync(port: &mut dyn serialport::SerialPort, retries: u8) -> anyhow::Result<()> {
    for attempt in 0..=retries {
        port.clear(ClearBuffer::Input)?;

        port.write_all(&[0x7F])?;
        port.flush()?;

        // Give the bootloader up to 2 s to respond; port read timeout is 500 ms so
        // up to 4 polls can happen within this window before giving up.
        let deadline = Instant::now() + Duration::from_millis(2000);
        loop {
            match read_byte(port) {
                Ok(ACK) => return Ok(()),
                Ok(NACK) if attempt < retries => break, // try again
                Ok(NACK) => bail!("Bootloader returned NACK during sync"),
                Ok(b) => bail!("Unexpected sync byte: 0x{:02X}", b),
                Err(_) if Instant::now() < deadline => continue,
                Err(_) if attempt < retries => break,
                Err(e) => return Err(e).context("Sync timeout"),
            }
        }
    }
    bail!("Sync failed after {} attempts", retries + 1)
}

/// Step 2: Get Chip ID command (0x02). Returns the 2-byte product ID.
pub fn get_id(port: &mut dyn serialport::SerialPort) -> anyhow::Result<u16> {
    port.write_all(&[0x02, 0xFD])?;
    port.flush()?;
    expect_ack(port).context("GetID command rejected")?;

    let n = read_byte(port)? as usize; // number of additional bytes - 1
    let mut id_bytes = vec![0u8; n + 1];
    port.read_exact(&mut id_bytes)?;
    expect_ack(port).context("GetID data ACK failed")?;

    if id_bytes.len() < 2 {
        bail!(
            "GetID returned too few bytes ({}), expected at least 2",
            id_bytes.len()
        );
    }

    let product_id = ((id_bytes[0] as u16) << 8) | (id_bytes[1] as u16);
    Ok(product_id)
}

/// Step 3: Extended Erase — mass erase all flash pages.
/// Uses command 0x44 with the special 0xFF 0xFF (mass erase) parameter.
///
/// `tick` is called on each 500 ms poll while waiting for the ACK.
/// Return `false` from `tick` to abort with a cancellation error.
pub fn extended_erase_all<F>(
    port: &mut dyn serialport::SerialPort,
    mut tick: F,
) -> anyhow::Result<()>
where
    F: FnMut() -> bool,
{
    port.write_all(&[0x44, 0xBB])?;
    port.flush()?;
    expect_ack(port).context("ExtendedErase command rejected")?;

    // Mass erase: 0xFF 0xFF + XOR checksum
    let data: &[u8] = &[0xFF, 0xFF, 0x00]; // checksum = 0xFF ^ 0xFF ^ 0x00 = 0x00
    port.write_all(data)?;
    port.flush()?;

    // Poll every 500 ms (port read timeout) until ACK or 30 s deadline.
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        match read_byte(port) {
            Ok(ACK) => return Ok(()),
            Ok(NACK) => bail!("Mass erase returned NACK"),
            Ok(b) => bail!("Unexpected erase response: 0x{:02X}", b),
            Err(e) if Instant::now() < deadline => {
                let kind = e.downcast_ref::<std::io::Error>().map(|e| e.kind());
                if kind == Some(std::io::ErrorKind::TimedOut) {
                    if !tick() {
                        bail!("Operation cancelled by user");
                    }
                    continue;
                }
                return Err(e).context("Erase error");
            }
            Err(e) => return Err(e).context("Erase timed out"),
        }
    }
}

/// Step 4: Write Memory — write up to 256 bytes to a given 32-bit address.
/// `data` must be 1–256 bytes; shorter writes are padded to a multiple of 4.
pub fn write_chunk(
    port: &mut dyn serialport::SerialPort,
    address: u32,
    data: &[u8],
) -> anyhow::Result<()> {
    if data.is_empty() || data.len() > 256 {
        bail!("write_chunk: data must be 1–256 bytes (got {})", data.len());
    }

    // WriteMemory command
    port.write_all(&[0x31, 0xCE])?;
    port.flush()?;
    expect_ack(port).context("WriteMemory command rejected")?;

    // Address: 4 bytes big-endian + XOR checksum
    let addr_bytes = address.to_be_bytes();
    let addr_checksum = xor_checksum(&addr_bytes);
    port.write_all(&addr_bytes)?;
    port.write_all(&[addr_checksum])?;
    port.flush()?;
    expect_ack(port).context("WriteMemory address ACK failed")?;

    // Pad data to multiple of 4 bytes (bootloader requirement)
    let mut padded: Vec<u8> = data.to_vec();
    while padded.len() % 4 != 0 {
        padded.push(0xFF);
    }

    // N (number of bytes - 1) + data + XOR checksum of (N || data)
    let n = (padded.len() - 1) as u8;
    let checksum = n ^ xor_checksum(&padded);
    port.write_all(&[n])?;
    port.write_all(&padded)?;
    port.write_all(&[checksum])?;
    port.flush()?;
    expect_ack(port).context("WriteMemory data ACK failed")?;

    Ok(())
}

/// Step 5: Go command — jump to application at given address.
pub fn go(port: &mut dyn serialport::SerialPort, address: u32) -> anyhow::Result<()> {
    port.write_all(&[0x21, 0xDE])?;
    port.flush()?;
    expect_ack(port).context("Go command rejected")?;

    let addr_bytes = address.to_be_bytes();
    let addr_checksum = xor_checksum(&addr_bytes);
    port.write_all(&addr_bytes)?;
    port.write_all(&[addr_checksum])?;
    port.flush()?;
    expect_ack(port).context("Go address ACK failed")?;

    Ok(())
}
