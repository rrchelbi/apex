use std::net::Ipv4Addr;

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

    pub fn write(&mut self, pb: &mut PacketBuffer) -> Result<()> {
        self.header.question_count = self.questions.len() as u16;
        self.header.answer_count = self.answers.len() as u16;
        self.header.authority_count = self.authorities.len() as u16;
        self.header.additional_count = self.additionals.len() as u16;

        self.header.write(pb)?;

        for question in &mut self.questions {
            question.write(pb)?;
        }

        for rec in self
            .answers
            .iter()
            .chain(&self.authorities)
            .chain(&self.additionals)
        {
            rec.write(pb)?;
        }

        Ok(())
    }

    /// Pick the first A record from the answer section.
    /// When multiple IPs are returned for a name, any one is equally valid.
    pub fn get_random_a(&self) -> Option<Ipv4Addr> {
        self.answers.iter().find_map(|record| match record {
            Record::A { addr, .. } => Some(*addr),
            _ => None,
        })
    }

    /// Iterator over all NS records in the authority section that are
    /// authoritative for `qname`, yielded as `(domain, host)` tuples.
    fn get_ns<'a>(&'a self, qname: &'a str) -> impl Iterator<Item = (&'a str, &'a str)> {
        self.authorities
            .iter()
            .filter_map(|record| match record {
                Record::NS { domain, host, .. } => Some((domain.as_str(), host.as_str())),
                _ => None,
            })
            .filter(move |(domain, _)| qname.ends_with(*domain))
    }

    /// Returns the IP of a name server from the authority section if its
    /// A record was bundled in the additional section.
    pub fn get_resolved_ns(&self, qname: &str) -> Option<Ipv4Addr> {
        self.get_ns(qname)
            .flat_map(|(_, host)| {
                self.additionals
                    .iter()
                    .filter_map(move |record| match record {
                        Record::A { domain, addr, .. } if domain == host => Some(*addr),
                        _ => None,
                    })
            })
            .next()
    }

    /// Returns the hostname of a name server from the authority section
    /// when no resolved IP is available — triggers a recursive NS lookup.
    pub fn get_unresolved_ns<'a>(&'a self, qname: &'a str) -> Option<&'a str> {
        self.get_ns(qname).map(|(_, host)| host).next()
    }
}
