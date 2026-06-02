mod header;
mod packet;
mod query;
mod question;
mod record;

pub use header::Header;
pub use packet::Packet;
pub use query::QueryType;
pub use question::Question;
pub use record::Record;

use anyhow::{Context, Result, bail};

const BUF_SIZE: usize = 512;

pub struct PacketBuffer {
    pub buf: [u8; BUF_SIZE],
    pub pos: usize,
}

impl PacketBuffer {
    pub fn new() -> Self {
        Self {
            buf: [0; BUF_SIZE],
            pos: 0,
        }
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn step(&mut self, steps: usize) -> Result<()> {
        self.pos = self
            .pos
            .checked_add(steps)
            .filter(|&p| p < BUF_SIZE)
            .context("step overflowed buffer")?;
        Ok(())
    }

    fn set(&mut self, pos: usize, val: u8) -> Result<()> {
        *self.buf.get_mut(pos).context("set out of bounds")? = val;
        Ok(())
    }

    fn set_u16(&mut self, pos: usize, val: u16) -> Result<()> {
        let [hi, lo] = val.to_be_bytes();
        self.set(pos, hi)?;
        self.set(pos + 1, lo)?;
        Ok(())
    }

    pub fn seek(&mut self, pos: usize) -> Result<()> {
        if pos < BUF_SIZE {
            self.pos = pos;
            Ok(())
        } else {
            bail!("seek out of bounds: {} >= {}", pos, BUF_SIZE)
        }
    }

    pub fn read(&mut self) -> Result<u8> {
        let byte = self
            .buf
            .get(self.pos)
            .copied()
            .context("read past end of buffer")?;
        self.pos += 1;
        Ok(byte)
    }

    /// Returns `None` if `pos` is out of bounds
    pub fn get(&self, pos: usize) -> Option<u8> {
        self.buf.get(pos).copied()
    }

    /// Returns `None` if the range `[start, start + len)` exceeds the buffer
    pub fn bytes(&self, start: usize, len: usize) -> Option<&[u8]> {
        let end = start.checked_add(len).filter(|&e| e < BUF_SIZE)?;
        Some(&self.buf[start..end])
    }

    pub fn read_u16(&mut self) -> Result<u16> {
        Ok(u16::from_be_bytes([self.read()?, self.read()?]))
    }

    pub fn read_u32(&mut self) -> Result<u32> {
        Ok(u32::from_be_bytes([
            self.read()?,
            self.read()?,
            self.read()?,
            self.read()?,
        ]))
    }

    pub fn read_qname(&mut self, outstr: &mut String) -> Result<()> {
        let mut pos = self.pos();
        let mut jumped = false;
        let mut jumps_performed = 0u8;
        const MAX_JUMPS: u8 = 5;
        let mut delim = "";

        loop {
            if jumps_performed > MAX_JUMPS {
                bail!("Limit of {} jumps exceeded", MAX_JUMPS);
            }

            let len = self.get(pos).context("failed to read label length")?;

            match len & 0xC0 {
                0xC0 => {
                    if !jumped {
                        self.seek(pos + 2)?;
                    }

                    let b2 = self.get(pos + 1).context("failed to read jump offset")? as u16;
                    let offset = (((len as u16) ^ 0xC0) << 8) | b2;
                    pos = offset as usize;

                    jumped = true;
                    jumps_performed += 1;
                }
                0x00 => {
                    pos += 1;

                    if len == 0 {
                        break;
                    }

                    outstr.push_str(delim);
                    outstr.push_str(
                        &String::from_utf8_lossy(
                            self.bytes(pos, len as usize)
                                .context("label slice out of bounds")?,
                        )
                        .to_lowercase(),
                    );

                    delim = ".";
                    pos += len as usize;
                }
                _ => bail!("invalid label flags: {:#04x}", len),
            }
        }

        if !jumped {
            self.seek(pos)?;
        }

        Ok(())
    }

    pub fn write(&mut self, val: u8) -> Result<()> {
        *self
            .buf
            .get_mut(self.pos)
            .context("write past end of buffer")? = val;
        self.pos += 1;
        Ok(())
    }

    pub fn write_u16(&mut self, val: u16) -> Result<()> {
        for byte in val.to_be_bytes() {
            self.write(byte)?;
        }
        Ok(())
    }

    pub fn write_u32(&mut self, val: u32) -> Result<()> {
        for byte in val.to_be_bytes() {
            self.write(byte)?;
        }
        Ok(())
    }

    pub fn write_qname(&mut self, qname: &str) -> Result<()> {
        for label in qname.split('.') {
            let len = label.len();
            if len > 0x3F {
                bail!("label exceeds 63 characters: {:?}", label);
            }
            self.write(len as u8)?;
            for &byte in label.as_bytes() {
                self.write(byte)?;
            }
        }
        self.write(0)?; // root label terminator
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_then_read_u16() {
        const DATA: u16 = 0x1234;
        let mut pb = PacketBuffer::new();
        pb.write_u16(DATA).unwrap();
        pb.seek(0).unwrap();
        assert_eq!(pb.read_u16().unwrap(), DATA);
    }

    #[test]
    fn test_write_then_read_u32() {
        const DATA: u32 = 0xDEADBEEF;
        let mut pb = PacketBuffer::new();
        pb.write_u32(DATA).unwrap();
        pb.seek(0).unwrap();
        assert_eq!(pb.read_u32().unwrap(), DATA);
    }

    #[test]
    fn test_read_past_end_returns_err() {
        let mut pb = PacketBuffer::new();
        pb.seek(511).unwrap();
        pb.read().unwrap(); // last valid byte
        assert!(pb.read().is_err()); // should fail
    }

    #[test]
    fn test_write_past_end_returns_err() {
        let mut pb = PacketBuffer::new();
        pb.seek(511).unwrap();
        pb.write(0xFF).unwrap(); // last valid byte
        assert!(pb.write(0xFF).is_err()); // should fail
    }

    #[test]
    fn test_get_out_of_bounds_returns_none() {
        let pb = PacketBuffer::new();
        assert!(pb.get(512).is_none());
        assert!(pb.get(511).is_some());
    }

    #[test]
    fn test_qname_roundtrip() {
        const QNAME: &'static str = "google.com";
        let mut pb = PacketBuffer::new();
        pb.write_qname(QNAME).unwrap();
        pb.seek(0).unwrap();
        let mut out = String::new();
        pb.read_qname(&mut out).unwrap();
        assert_eq!(out, QNAME);
    }
}
