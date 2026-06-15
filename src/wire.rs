use crate::{
    message::{DomainName, Message, MessageKind, Question, ResourceRecordData},
    types::{Opcode, QClass, QType, RClass, RType, ResponseCode},
};

#[derive(Debug, Copy, Clone)]
pub struct ToWireResult {
    pub truncation: bool,
}

#[derive(Debug, Clone)]
pub enum ToWireError {
    MaxSizeTooSmall,
    TooManyQuestionRecords,
    TooManyAnswerRecords,
    TooManyAuthorityRecords,
    TooManyAdditionalRecords,
    LabelTooLong(Vec<u8>),
}

pub trait ToWire {
    fn to_wire(&self, buf: &mut Vec<u8>, max_size: usize) -> Result<ToWireResult, ToWireError>;
}

trait ToWireEntity {
    fn to_wire_entity(&self, buf: &mut Vec<u8>) -> Result<(), ToWireError>;
}

fn write_header(
    buf: &mut [u8],
    message: &Message,
    question_count: u16,
    answer_count: u16,
    authority_count: u16,
    additional_count: u16,
    truncation: bool,
) {
    buf[..2].copy_from_slice(&message.id.to_be_bytes());
    buf[2..4].copy_from_slice(&extract_header_info(message, truncation).to_be_bytes());
    buf[4..6].copy_from_slice(&question_count.to_be_bytes());
    buf[6..8].copy_from_slice(&answer_count.to_be_bytes());
    buf[8..10].copy_from_slice(&authority_count.to_be_bytes());
    buf[10..12].copy_from_slice(&additional_count.to_be_bytes());
}

fn extract_header_info(message: &Message, truncation: bool) -> u16 {
    let mut info = u16::from(message.opcode) << 11
        | u16::from(truncation) << 9
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
            | u16::from(response_code) & 0x000F
    }

    info
}

impl ToWire for Message {
    fn to_wire(&self, buf: &mut Vec<u8>, max_size: usize) -> Result<ToWireResult, ToWireError> {
        let zero = buf.len();

        // The header needs 12 octets
        if max_size < 12 {
            return Err(ToWireError::MaxSizeTooSmall);
        }

        if self.questions.len() > u16::MAX as usize {
            return Err(ToWireError::TooManyQuestionRecords);
        }

        if let MessageKind::Response {
            answer,
            authority,
            additional,
            ..
        } = &self.kind
        {
            if answer.len() > u16::MAX as usize {
                return Err(ToWireError::TooManyAnswerRecords);
            }

            if authority.len() > u16::MAX as usize {
                return Err(ToWireError::TooManyAuthorityRecords);
            }

            if additional.len() > u16::MAX as usize {
                return Err(ToWireError::TooManyAdditionalRecords);
            }
        }

        // Reserve 12 octets for the header section
        buf.resize(zero + 12, 0u8);

        let mut truncation = false;

        // Question section
        let mut question_count = 0u16;
        for question in &self.questions {
            let current_size = buf.len();
            question.to_wire_entity(buf).inspect_err(|_| {
                buf.truncate(zero);
            })?;
            if buf.len() - zero > max_size {
                buf.truncate(current_size);
                truncation = true;
                break;
            }
            question_count += 1;
        }

        // TODO: Answer, Authority and Additional sections in responses

        // Populate the header with the correct information
        write_header(
            &mut buf[zero..zero + 12],
            self,
            question_count,
            0u16,
            0u16,
            0u16,
            truncation,
        );

        Ok(ToWireResult { truncation })
    }
}

impl ToWireEntity for Question {
    fn to_wire_entity(&self, buf: &mut Vec<u8>) -> Result<(), ToWireError> {
        self.qname.to_wire_entity(buf)?;
        buf.extend_from_slice(&u16::from(self.qtype).to_be_bytes());
        buf.extend_from_slice(&u16::from(self.qclass).to_be_bytes());
        Ok(())
    }
}

impl ToWireEntity for DomainName {
    fn to_wire_entity(&self, buf: &mut Vec<u8>) -> Result<(), ToWireError> {
        self.iter().try_for_each(|label| {
            if label.len() >= 64 {
                Err(ToWireError::LabelTooLong(label.clone()))
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
