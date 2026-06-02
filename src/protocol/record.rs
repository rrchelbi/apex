use anyhow::Result;
use std::net::{Ipv4Addr, Ipv6Addr};
use tracing;

use super::{PacketBuffer, QueryType};

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Record {
    Unknown {
        domain: String,
        qtype: u16,
        data_len: u16,
        ttl: u32,
    },
    A {
        domain: String,
        addr: Ipv4Addr,
        ttl: u32,
    },
    NS {
        domain: String,
        host: String,
        ttl: u32,
    },
    CNAME {
        domain: String,
        host: String,
        ttl: u32,
    },
    MX {
        domain: String,
        priority: u16,
        host: String,
        ttl: u32,
    },
    AAAA {
        domain: String,
        addr: Ipv6Addr,
        ttl: u32,
    },
}

impl Record {
    pub fn read(pb: &mut PacketBuffer) -> Result<Self> {
        let mut domain = String::new();
        pb.read_qname(&mut domain)?;

        let qtype_num = pb.read_u16()?;
        let qtype = QueryType::from(qtype_num);
        let _class = pb.read_u16()?;
        let ttl = pb.read_u32()?;
        let data_len = pb.read_u16()?;

        match qtype {
            QueryType::A => {
                let addr = Ipv4Addr::from(pb.read_u32()?);
                Ok(Self::A { domain, addr, ttl })
            }
            QueryType::AAAA => {
                let addr = Ipv6Addr::from([
                    pb.read_u16()?,
                    pb.read_u16()?,
                    pb.read_u16()?,
                    pb.read_u16()?,
                    pb.read_u16()?,
                    pb.read_u16()?,
                    pb.read_u16()?,
                    pb.read_u16()?,
                ]);
                Ok(Self::AAAA { domain, addr, ttl })
            }

            QueryType::NS => {
                let mut host = String::new();
                pb.read_qname(&mut host)?;
                Ok(Self::NS { domain, host, ttl })
            }

            QueryType::CNAME => {
                let mut host = String::new();
                pb.read_qname(&mut host)?;
                Ok(Self::CNAME { domain, host, ttl })
            }

            QueryType::MX => {
                let priority = pb.read_u16()?;
                let mut host = String::new();
                pb.read_qname(&mut host)?;
                Ok(Self::MX {
                    domain,
                    priority,
                    host,
                    ttl,
                })
            }
            QueryType::Unknown(_) => {
                pb.step(data_len as usize)?;
                Ok(Self::Unknown {
                    domain,
                    qtype: qtype_num,
                    data_len,
                    ttl,
                })
            }
        }
    }

    pub fn write(&self, pb: &mut PacketBuffer) -> Result<usize> {
        let start_pos = pb.pos();

        match self {
            Self::A { domain, addr, ttl } => {
                pb.write_qname(domain)?;
                pb.write_u16(QueryType::A.into())?;
                pb.write_u16(1)?; // class IN
                pb.write_u32(*ttl)?;
                pb.write_u16(4)?; // ipv4 = 4 bytes
                for byte in addr.octets() {
                    pb.write(byte)?;
                }
            }

            Self::AAAA { domain, addr, ttl } => {
                pb.write_qname(domain)?;
                pb.write_u16(QueryType::AAAA.into())?;
                pb.write_u16(1)?; // class IN
                pb.write_u32(*ttl)?;
                pb.write_u16(16)?; // ipv6 = 16 bytes
                for segment in addr.segments() {
                    pb.write_u16(segment)?;
                }
            }

            Self::NS { domain, host, ttl } => {
                pb.write_qname(domain)?;
                pb.write_u16(QueryType::NS.into())?;
                pb.write_u16(1)?; // class IN
                pb.write_u32(*ttl)?;
                write_with_length_prefix(pb, |buf| buf.write_qname(host))?;
            }

            Self::CNAME { domain, host, ttl } => {
                pb.write_qname(domain)?;
                pb.write_u16(QueryType::CNAME.into())?;
                pb.write_u16(1)?; // class IN
                pb.write_u32(*ttl)?;
                write_with_length_prefix(pb, |buf| buf.write_qname(host))?;
            }

            Self::MX {
                domain,
                priority,
                host,
                ttl,
            } => {
                pb.write_qname(domain)?;
                pb.write_u16(QueryType::MX.into())?;
                pb.write_u16(1)?; // class IN
                pb.write_u32(*ttl)?;
                write_with_length_prefix(pb, |buf| {
                    buf.write_u16(*priority)?;
                    buf.write_qname(host)
                })?;
            }

            Self::Unknown { .. } => {
                tracing::warn!("skipping unknown record: {:?}", self);
            }
        }

        Ok(pb.pos() - start_pos)
    }
}

/// Writes a u16 length-prefixed block. Reserves 2 bytes for the length,
/// runs the writer, then backfills the actual byte count.
fn write_with_length_prefix<F>(buffer: &mut PacketBuffer, writer: F) -> Result<()>
where
    F: FnOnce(&mut PacketBuffer) -> Result<()>,
{
    let len_pos = buffer.pos();
    buffer.write_u16(0)?; // placeholder
    writer(buffer)?;
    let data_len = buffer.pos() - (len_pos + 2);
    buffer.set_u16(len_pos, data_len as u16)?;
    Ok(())
}
