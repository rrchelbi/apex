use super::PacketBuffer;
use super::QueryType;

use anyhow::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Question {
    pub name: String,
    pub qtype: QueryType,
}

impl Question {
    pub fn new(name: impl Into<String>, qtype: QueryType) -> Self {
        Self {
            name: name.into(),
            qtype,
        }
    }

    pub fn read(&mut self, buffer: &mut PacketBuffer) -> Result<()> {
        buffer.read_qname(&mut self.name)?;
        self.qtype = QueryType::from(buffer.read_u16()?);
        let _class = buffer.read_u16()?; // class field, ignored for now
        Ok(())
    }

    pub fn write(&mut self, pb: &mut PacketBuffer) -> Result<()> {
        pb.write_qname(&self.name)?;
        pb.write_u16(self.qtype.into())?;
        pb.write_u16(1)?; // class IN
        Ok(())
    }
}
