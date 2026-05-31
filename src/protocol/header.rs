use super::PacketBuffer;
use anyhow::Result;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum ResultCode {
    #[default]
    NOERROR = 0,
    FORMERR = 1,
    SERVFAIL = 2,
    NXDOMAIN = 3,
    NOTIMP = 4,
    REFUSED = 5,
}

impl From<u8> for ResultCode {
    fn from(num: u8) -> Self {
        match num {
            1 => Self::FORMERR,
            2 => Self::SERVFAIL,
            3 => Self::NXDOMAIN,
            4 => Self::NOTIMP,
            5 => Self::REFUSED,
            _ => Self::NOERROR,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Header {
    pub id: u16,
    pub is_response: bool,
    pub opcode: u8,
    pub authoritative: bool,
    pub truncated: bool,
    pub recursion_desired: bool,
    pub recursion_available: bool,
    /// Reserved for future use. Must be zero in all queries and responses.
    /// [rfc1035, section 4.1.1](https://www.rfc-editor.org/info/rfc1035/#section-4.1.1)
    pub z: bool,
    pub authenticated_data: bool,
    pub checking_disabled: bool,
    pub result_code: ResultCode,
    pub question_count: u16,
    pub answer_count: u16,
    pub authority_count: u16,
    pub additional_count: u16,
}

impl Header {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read(&mut self, buffer: &mut PacketBuffer) -> Result<()> {
        self.id = buffer.read_u16()?;

        let flags = buffer.read_u16()?;
        let [a, b] = flags.to_be_bytes();

        self.is_response = (a & 0b1000_0000) != 0;
        self.opcode = (a & 0b0111_1000) >> 3;
        self.authoritative = (a & 0b0000_0100) != 0;
        self.truncated = (a & 0b0000_0010) != 0;
        self.recursion_desired = (a & 0b0000_0001) != 0;

        self.recursion_available = (b & 0b1000_0000) != 0;
        self.z = (b & 0b0100_0000) != 0;
        self.authenticated_data = (b & 0b0010_0000) != 0;
        self.checking_disabled = (b & 0b0001_0000) != 0;
        self.result_code = ResultCode::from(b & 0b0000_1111);

        self.question_count = buffer.read_u16()?;
        self.answer_count = buffer.read_u16()?;
        self.authority_count = buffer.read_u16()?;
        self.additional_count = buffer.read_u16()?;

        Ok(())
    }
}
