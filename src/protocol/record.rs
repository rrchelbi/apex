use std::net::Ipv4Addr;
use anyhow::Result;

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
                Ok(Self::Unknown { domain, qtype: qtype_num, data_len, ttl })
            }
        }
    }
}
