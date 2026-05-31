use super::{Header, PacketBuffer, QueryType, Question, Record};
use anyhow::Result;

#[derive(Clone, Debug, Default)]
pub struct Packet {
    pub header: Header,
    pub questions: Vec<Question>,
    pub answers: Vec<Record>,
    pub authorities: Vec<Record>,
    pub additionals: Vec<Record>, // "resources" → "additionals" matches RFC 1035
}

impl TryFrom<&mut PacketBuffer> for Packet {
    type Error = anyhow::Error;

    fn try_from(buffer: &mut PacketBuffer) -> Result<Self> {
        let mut packet = Self::new();
        packet.header.read(buffer)?;

        packet.questions = (0..packet.header.question_count)
            .map(|_| {
                let mut q = Question::new("", QueryType::Unknown(0));
                q.read(buffer)?;
                Ok(q)
            })
            .collect::<Result<_>>()?;

        packet.answers = (0..packet.header.answer_count)
            .map(|_| Record::read(buffer))
            .collect::<Result<_>>()?;

        packet.authorities = (0..packet.header.authority_count)
            .map(|_| Record::read(buffer))
            .collect::<Result<_>>()?;

        packet.additionals = (0..packet.header.additional_count)
            .map(|_| Record::read(buffer))
            .collect::<Result<_>>()?;

        Ok(packet)
    }
}

impl Packet {
    pub fn new() -> Self {
        Self::default()
    }
}
