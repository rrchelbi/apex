use anyhow::Result;
use std::net::Ipv4Addr;
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
}

impl Record {
    pub fn read(buffer: &mut PacketBuffer) -> Result<Self> {
        let mut domain = String::new();
        buffer.read_qname(&mut domain)?;

        let qtype_num = buffer.read_u16()?;
        let qtype = QueryType::from(qtype_num);
        let _class = buffer.read_u16()?;
        let ttl = buffer.read_u32()?;
        let data_len = buffer.read_u16()?;

        match qtype {
            QueryType::A => {
                let addr = Ipv4Addr::from(buffer.read_u32()?);
                Ok(Self::A { domain, addr, ttl })
            }
            QueryType::Unknown(_) => {
                buffer.step(data_len as usize)?;
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
                pb.write_u16(4)?; // ipv4 addr length in bytes
                for byte in addr.octets() {
                    pb.write(byte)?;
                }
            }
            Self::Unknown { .. } => {
                tracing::warn!("skipping unknown record: {:?}", self);
            }
        }

        Ok(pb.pos() - start_pos)
    }
}
