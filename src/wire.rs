use crate::{
    message::{Message, MessageKind, Question, ResourceRecordData},
    types::{DomainName, Opcode, QClass, QType, RClass, RType, ResponseCode},
};

pub trait ToWire {
    fn to_wire(&self, buf: &mut Vec<u8>);
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
    fn to_wire(&self, buf: &mut Vec<u8>) {
        // Header section
        buf.extend_from_slice(&self.id.to_be_bytes());

        let info = extract_header_info(self);
        buf.extend_from_slice(&info.to_be_bytes());

        let qd_count = u16::try_from(self.questions.len()).expect("Too many question records"); // TODO: error handling
        buf.extend_from_slice(&qd_count.to_be_bytes());

        let (an_count, ns_count, ar_count) = match &self.kind {
            MessageKind::Query => (0u16, 0u16, 0u16),
            MessageKind::Response {
                answer,
                authority,
                additional,
                ..
            } => (
                u16::try_from(answer.len()).expect("Too many answer records"),
                u16::try_from(authority.len()).expect("Too many authority records"),
                u16::try_from(additional.len()).expect("Too many additional records"),
            ),
        };
        buf.extend_from_slice(&an_count.to_be_bytes());
        buf.extend_from_slice(&ns_count.to_be_bytes());
        buf.extend_from_slice(&ar_count.to_be_bytes());

	// Question section
        self.questions
            .iter()
            .for_each(|question| question.to_wire(buf));

        // TODO: Answer, Authority and Additional sections in responses
    }
}

impl ToWire for Question {
    fn to_wire(&self, buf: &mut Vec<u8>) {
        // TODO: Append qname
        self.qname.to_wire(buf);
        buf.extend_from_slice(&u16::from(self.qtype).to_be_bytes());
        buf.extend_from_slice(&u16::from(self.qclass).to_be_bytes());
    }
}

impl ToWire for DomainName {
    fn to_wire(&self, buf: &mut Vec<u8>) {
        self.iter().for_each(|label| {
            let len = u8::try_from(label.len()).expect("Label too long");
            if len >= 64 {
                panic!("Label too long");
            }
            buf.push(len);
            buf.extend_from_slice(label);
        });
        buf.push(0u8);
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
