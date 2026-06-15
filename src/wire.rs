use crate::{
    message::{Message, MessageKind, Question, ResourceRecordData},
    types::{DomainName, Opcode, QClass, QType, RClass, RType, ResponseCode},
};

#[derive(Debug, Copy, Clone)]
pub enum ToWireError<'a> {
    TooManyQuestionRecords,
    TooManyAnswerRecords,
    TooManyAuthorityRecords,
    TooManyAdditionalRecords,
    LabelTooLong(&'a [u8]),
}

pub trait ToWire {
    fn to_wire(&self, buf: &mut Vec<u8>) -> Result<(), ToWireError>;
}

fn extract_header_info(message: &Message) -> u16 {
    let mut info = u16::from(message.opcode) << 11
        | u16::from(message.truncation) << 9
        | u16::from(message.recursion_desired) << 8;

    if let MessageKind::Response {
        authoritative,
        recursion_available,
        response_code,
        ..
    } = message.kind
    {
        info |= 1u16 << 15
            | u16::from(authoritative) << 10
            | u16::from(recursion_available) << 7
            | u16::from(response_code)
    }

    info
}

impl ToWire for Message {
    fn to_wire(&self, buf: &mut Vec<u8>) -> Result<(), ToWireError<'_>> {
        // Header section
        buf.extend_from_slice(&self.id.to_be_bytes());

        let info = extract_header_info(self);
        buf.extend_from_slice(&info.to_be_bytes());

        let qd_count =
            u16::try_from(self.questions.len()).map_err(|_| ToWireError::TooManyQuestionRecords)?;
        buf.extend_from_slice(&qd_count.to_be_bytes());

        let (an_count, ns_count, ar_count) = match &self.kind {
            MessageKind::Query => Ok((0u16, 0u16, 0u16)),
            MessageKind::Response {
                answer,
                authority,
                additional,
                ..
            } => Ok((
                u16::try_from(answer.len()).map_err(|_| ToWireError::TooManyAnswerRecords)?,
                u16::try_from(authority.len()).map_err(|_| ToWireError::TooManyAuthorityRecords)?,
                u16::try_from(additional.len())
                    .map_err(|_| ToWireError::TooManyAdditionalRecords)?,
            )),
        }?;
        buf.extend_from_slice(&an_count.to_be_bytes());
        buf.extend_from_slice(&ns_count.to_be_bytes());
        buf.extend_from_slice(&ar_count.to_be_bytes());

        // Question section
        self.questions
            .iter()
            .try_for_each(|question| question.to_wire(buf))?;

        // TODO: Answer, Authority and Additional sections in responses
        Ok(())
    }
}

impl ToWire for Question {
    fn to_wire(&self, buf: &mut Vec<u8>) -> Result<(), ToWireError> {
        self.qname.to_wire(buf)?;
        buf.extend_from_slice(&u16::from(self.qtype).to_be_bytes());
        buf.extend_from_slice(&u16::from(self.qclass).to_be_bytes());
        Ok(())
    }
}

impl ToWire for DomainName {
    fn to_wire(&self, buf: &mut Vec<u8>) -> Result<(), ToWireError> {
        self.iter().try_for_each(|label| {
            if label.len() >= 64 {
                Err(ToWireError::LabelTooLong(&label))
            } else {
                buf.push(label.len() as u8);
                buf.extend_from_slice(label);
                Ok(())
            }
        })?;
        buf.push(0u8);
        Ok(())
    }
}

impl From<Opcode> for u8 {
    fn from(opcode: Opcode) -> u8 {
        match opcode {
            Opcode::Standard => 0u8,
            Opcode::Inverse => 1u8,
            Opcode::Status => 2u8,
            Opcode::Unknown(o) => o,
        }
    }
}

impl From<Opcode> for u16 {
    fn from(opcode: Opcode) -> u16 {
        u8::from(opcode).into()
    }
}

impl From<ResponseCode> for u8 {
    fn from(response_code: ResponseCode) -> u8 {
        match response_code {
            ResponseCode::NoError => 0u8,
            ResponseCode::FormatError => 1u8,
            ResponseCode::ServerFailure => 2u8,
            ResponseCode::NameError => 3u8,
            ResponseCode::NotImplemented => 4u8,
            ResponseCode::Refused => 5u8,
            ResponseCode::Unknown(c) => c,
        }
    }
}

impl From<ResponseCode> for u16 {
    fn from(response_code: ResponseCode) -> u16 {
        u8::from(response_code).into()
    }
}

impl From<RClass> for u16 {
    fn from(class: RClass) -> u16 {
        match class {
            RClass::IN => 1u16,
            RClass::CS => 2u16,
            RClass::CH => 3u16,
            RClass::HS => 4u16,
            RClass::Unknown(c) => c,
        }
    }
}

impl From<QClass> for u16 {
    fn from(qclass: QClass) -> u16 {
        match qclass {
            QClass::RClass(c) => u16::from(c),
            QClass::Any => 255u16,
        }
    }
}

impl From<RType> for u16 {
    fn from(qtype: RType) -> u16 {
        match qtype {
            RType::A => 1u16,
            RType::NS => 2u16,
            RType::MD => 3u16,
            RType::MF => 4u16,
            RType::CNAME => 5u16,
            RType::SOA => 6u16,
            RType::MB => 7u16,
            RType::MG => 8u16,
            RType::MR => 9u16,
            RType::NULL => 10u16,
            RType::WKS => 11u16,
            RType::PTR => 12u16,
            RType::HINFO => 13u16,
            RType::MINFO => 14u16,
            RType::MX => 15u16,
            RType::TXT => 16u16,
            RType::Unknown(t) => t,
        }
    }
}

impl From<QType> for u16 {
    fn from(qtype: QType) -> u16 {
        match qtype {
            QType::RType(t) => u16::from(t),
            QType::AXFR => 252u16,
            QType::MAILB => 253u16,
            QType::MAILA => 254u16,
            QType::Any => 255u16,
        }
    }
}

impl From<&ResourceRecordData> for u16 {
    fn from(record: &ResourceRecordData) -> u16 {
        RType::from(record).into()
    }
}
