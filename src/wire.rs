use crate::{
    message::{DomainName, Message, MessageKind, Question, ResourceRecordData},
    types::{Opcode, QClass, QType, RClass, RType, ResponseCode},
};

#[derive(Debug, Copy, Clone)]
pub struct ToWireResult {
    pub truncation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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
    let mut info = (u16::from(message.opcode) & 0x000Fu16) << 11
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
            | u16::from(response_code) & 0x000Fu16
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::ResourceRecord;

    macro_rules! common_builder_methods {
        () => {
            fn id(mut self, id: u16) -> Self {
                self.id = id;
                self
            }
            fn opcode(mut self, opcode: Opcode) -> Self {
                self.opcode = opcode;
                self
            }
            fn recursion_desired(mut self, v: bool) -> Self {
                self.recursion_desired = v;
                self
            }
            fn questions(mut self, q: Vec<Question>) -> Self {
                self.questions = q;
                self
            }
        };
    }

    struct QueryBuilder {
        id: u16,
        recursion_desired: bool,
        opcode: Opcode,
        questions: Vec<Question>,
    }

    impl QueryBuilder {
        fn new() -> Self {
            Self {
                id: 0,
                recursion_desired: false,
                opcode: Opcode::Standard,
                questions: vec![],
            }
        }

        common_builder_methods!();

        fn build(self) -> Message {
            Message {
                id: self.id,
                kind: MessageKind::Query,
                opcode: self.opcode,
                recursion_desired: self.recursion_desired,
                questions: self.questions,
            }
        }
    }

    struct ResponseBuilder {
        id: u16,
        authoritative: bool,
        recursion_available: bool,
        recursion_desired: bool,
        response_code: ResponseCode,
        opcode: Opcode,
        questions: Vec<Question>,
        answer: Vec<ResourceRecord>,
        authority: Vec<ResourceRecord>,
        additional: Vec<ResourceRecord>,
    }

    impl ResponseBuilder {
        fn new() -> Self {
            Self {
                id: 0,
                authoritative: false,
                recursion_available: false,
                recursion_desired: false,
                response_code: ResponseCode::NoError,
                opcode: Opcode::Standard,
                questions: vec![],
                answer: vec![],
                authority: vec![],
                additional: vec![],
            }
        }

        common_builder_methods!();

        fn authoritative(mut self, v: bool) -> Self {
            self.authoritative = v;
            self
        }

        fn recursion_available(mut self, v: bool) -> Self {
            self.recursion_available = v;
            self
        }

        fn response_code(mut self, v: ResponseCode) -> Self {
            self.response_code = v;
            self
        }

        fn answer(mut self, v: Vec<ResourceRecord>) -> Self {
            self.answer = v;
            self
        }

        fn authority(mut self, v: Vec<ResourceRecord>) -> Self {
            self.authority = v;
            self
        }

        fn additional(mut self, v: Vec<ResourceRecord>) -> Self {
            self.additional = v;
            self
        }

        fn build(self) -> Message {
            Message {
                id: self.id,
                kind: MessageKind::Response {
                    authoritative: self.authoritative,
                    recursion_available: self.recursion_available,
                    response_code: self.response_code,
                    answer: self.answer,
                    authority: self.authority,
                    additional: self.additional,
                },
                opcode: self.opcode,
                recursion_desired: self.recursion_desired,
                questions: self.questions,
            }
        }
    }

    mod errors {
        use super::*;

        #[test]
        fn check_max_size_too_small() {
            let query = QueryBuilder::new().build();
            let mut buf = Vec::<u8>::new();
            let result = query.to_wire(&mut buf, 11);

            assert!(result.is_err());
            assert_eq!(result.unwrap_err(), ToWireError::MaxSizeTooSmall);
            assert_eq!(buf.len(), 0);
        }

        #[test]
        fn check_too_many_question_records() {
            let question = Question {
                qname: vec![],
                qtype: QType::RType(RType::NS),
                qclass: QClass::RClass(RClass::IN),
            };
            let question_number = u16::MAX as usize + 1;

            let query = ResponseBuilder::new()
                .questions(vec![question; question_number])
                .build();
            let mut buf = Vec::<u8>::new();
            let result = query.to_wire(&mut buf, 12 + question_number * 5);

            assert!(result.is_err());
            assert_eq!(result.unwrap_err(), ToWireError::TooManyQuestionRecords);
            assert_eq!(buf.len(), 0);
        }

        #[test]
        fn check_too_many_answer_records() {
            let answer = ResourceRecord {
                name: vec![],
                class: RClass::IN,
                ttl: 100,
                rdata: ResourceRecordData::NS(vec![]),
            };
            let answer_number = u16::MAX as usize + 1;

            let response = ResponseBuilder::new()
                .answer(vec![answer; answer_number])
                .build();
            let mut buf = Vec::<u8>::new();
            let result = response.to_wire(&mut buf, 12 + answer_number * 5);

            assert!(result.is_err());
            assert_eq!(result.unwrap_err(), ToWireError::TooManyAnswerRecords);
            assert_eq!(buf.len(), 0);
        }

        #[test]
        fn check_too_many_authority_records() {
            let authority = ResourceRecord {
                name: vec![],
                class: RClass::IN,
                ttl: 100,
                rdata: ResourceRecordData::NS(vec![]),
            };
            let authority_number = u16::MAX as usize + 1;

            let response = ResponseBuilder::new()
                .authority(vec![authority; authority_number])
                .build();
            let mut buf = Vec::<u8>::new();
            let result = response.to_wire(&mut buf, 12 + authority_number * 5);

            assert!(result.is_err());
            assert_eq!(result.unwrap_err(), ToWireError::TooManyAuthorityRecords);
            assert_eq!(buf.len(), 0);
        }

        #[test]
        fn check_too_many_additional_records() {
            let additional = ResourceRecord {
                name: vec![],
                class: RClass::IN,
                ttl: 100,
                rdata: ResourceRecordData::NS(vec![]),
            };
            let additional_number = u16::MAX as usize + 1;

            let response = ResponseBuilder::new()
                .additional(vec![additional; additional_number])
                .build();
            let mut buf = Vec::<u8>::new();
            let result = response.to_wire(&mut buf, 12 + additional_number * 5);

            assert!(result.is_err());
            assert_eq!(result.unwrap_err(), ToWireError::TooManyAdditionalRecords);
            assert_eq!(buf.len(), 0);
        }

        #[test]
        fn check_label_too_long() {
            let label = vec![b'a'; 64];
            let question = Question {
                qname: vec![label.clone()],
                qtype: QType::RType(RType::NS),
                qclass: QClass::RClass(RClass::IN),
            };
            let query = QueryBuilder::new().questions(vec![question]).build();
            let mut buf = Vec::<u8>::new();
            let result = query.to_wire(&mut buf, 512);

            assert!(result.is_err());
            assert_eq!(result.unwrap_err(), ToWireError::LabelTooLong(label));
            assert_eq!(buf.len(), 0);
        }
    }

    mod header {
        use super::*;

        #[test]
        fn check_id_query() {
            let query = QueryBuilder::new().id(0xABCDu16).build();
            let mut buf = Vec::<u8>::new();
            let result = query.to_wire(&mut buf, 512);

            assert!(result.is_ok());
            assert_eq!(buf[0], 0xABu8);
            assert_eq!(buf[1], 0xCDu8);
        }

        #[test]
        fn check_id_response() {
            let response = ResponseBuilder::new().id(0xABCDu16).build();
            let mut buf = Vec::<u8>::new();
            let result = response.to_wire(&mut buf, 512);

            assert!(result.is_ok());
            assert_eq!(buf[0], 0xABu8);
            assert_eq!(buf[1], 0xCDu8);
        }

        // Check kind
        #[test]
        fn check_kind_query() {
            let query = QueryBuilder::new().build();
            let mut buf = Vec::<u8>::new();
            let result = query.to_wire(&mut buf, 512);

            assert!(result.is_ok());
            assert_eq!(buf[2] & 0b1000_0000u8, 0b0000_0000u8);
        }

        #[test]
        fn check_kind_response() {
            let response = ResponseBuilder::new().build();
            let mut buf = Vec::<u8>::new();
            let result = response.to_wire(&mut buf, 512);

            assert!(result.is_ok());
            assert_eq!(buf[2] & 0b1000_0000u8, 0b1000_0000u8);
        }

        fn check_opcode(opcode: Opcode, expected: u8) {
            let query = QueryBuilder::new().opcode(opcode).build();
            let mut buf = Vec::<u8>::new();
            let result = query.to_wire(&mut buf, 512);

            assert!(result.is_ok());
            assert_eq!((buf[2] & 0b0111_1000u8) >> 3, expected);
        }

        #[test]
        fn check_opcode_standard() {
            check_opcode(Opcode::Standard, 0u8);
        }

        #[test]
        fn check_opcode_inverse() {
            check_opcode(Opcode::Inverse, 1u8);
        }

        #[test]
        fn check_opcode_status() {
            check_opcode(Opcode::Status, 2u8);
        }

        #[test]
        fn check_opcode_unknown() {
            check_opcode(Opcode::Unknown(10u8), 10u8);
        }

        #[test]
        fn check_opcode_response() {
            let query = ResponseBuilder::new().opcode(Opcode::Inverse).build();
            let mut buf = Vec::<u8>::new();
            let result = query.to_wire(&mut buf, 512);

            assert!(result.is_ok());
            assert_eq!((buf[2] & 0b0111_1000u8) >> 3, 1u8);
        }

        #[test]
        fn check_authoritative_false() {
            let response = ResponseBuilder::new().authoritative(false).build();
            let mut buf = Vec::<u8>::new();
            let result = response.to_wire(&mut buf, 512);

            assert!(result.is_ok());
            assert_eq!(buf[2] & 0b0000_0100u8, 0b0000_0000u8);
        }

        #[test]
        fn check_authoritative_true() {
            let response = ResponseBuilder::new().authoritative(true).build();
            let mut buf = Vec::<u8>::new();
            let result = response.to_wire(&mut buf, 512);

            assert!(result.is_ok());
            assert_eq!(buf[2] & 0b0000_0100u8, 0b0000_0100u8);
        }

        #[test]
        fn check_recursion_desired_false() {
            let query = QueryBuilder::new().recursion_desired(false).build();
            let mut buf = Vec::<u8>::new();
            let result = query.to_wire(&mut buf, 512);

            assert!(result.is_ok());
            assert_eq!(buf[2] & 0b0000_0001u8, 0b0000_0000u8);
        }

        #[test]
        fn check_recursion_desired_true() {
            let response = ResponseBuilder::new().recursion_desired(true).build();
            let mut buf = Vec::<u8>::new();
            let result = response.to_wire(&mut buf, 512);

            assert!(result.is_ok());
            assert_eq!(buf[2] & 0b0000_0001u8, 0b0000_0001u8);
        }

        #[test]
        fn check_recursion_available_false() {
            let response = ResponseBuilder::new().recursion_available(false).build();
            let mut buf = Vec::<u8>::new();
            let result = response.to_wire(&mut buf, 512);

            assert!(result.is_ok());
            assert_eq!(buf[3] & 0b1000_0000u8, 0b0000_0000u8);
        }

        #[test]
        fn check_recursion_available_true() {
            let response = ResponseBuilder::new().recursion_available(true).build();
            let mut buf = Vec::<u8>::new();
            let result = response.to_wire(&mut buf, 512);

            assert!(result.is_ok());
            assert_eq!(buf[3] & 0b1000_0000u8, 0b1000_0000u8);
        }

        #[test]
        fn check_z_query() {
            let query = QueryBuilder::new().build();
            let mut buf = Vec::<u8>::new();
            let result = query.to_wire(&mut buf, 512);

            assert!(result.is_ok());
            assert_eq!(buf[3] & 0b0111_0000u8, 0b0000_0000u8);
        }

        #[test]
        fn check_z_response() {
            let response = ResponseBuilder::new().build();
            let mut buf = Vec::<u8>::new();
            let result = response.to_wire(&mut buf, 512);

            assert!(result.is_ok());
            assert_eq!(buf[3] & 0b0111_0000u8, 0b0000_0000u8);
        }

        fn check_response_code(response_code: ResponseCode, expected: u8) {
            let response = ResponseBuilder::new().response_code(response_code).build();
            let mut buf = Vec::<u8>::new();
            let result = response.to_wire(&mut buf, 512);

            assert!(result.is_ok());
            assert_eq!((buf[3] & 0b0000_1111u8), expected);
        }

        #[test]
        fn check_response_code_no_error() {
            check_response_code(ResponseCode::NoError, 0u8);
        }

        #[test]
        fn check_response_code_format_error() {
            check_response_code(ResponseCode::FormatError, 1u8);
        }

        #[test]
        fn check_response_code_server_failure() {
            check_response_code(ResponseCode::ServerFailure, 2u8);
        }

        #[test]
        fn check_response_code_name_error() {
            check_response_code(ResponseCode::NameError, 3u8);
        }

        #[test]
        fn check_response_code_not_implemented_error() {
            check_response_code(ResponseCode::NotImplemented, 4u8);
        }

        #[test]
        fn check_response_code_refused() {
            check_response_code(ResponseCode::Refused, 5u8);
        }

        #[test]
        fn check_response_code_unknown() {
            check_response_code(ResponseCode::Unknown(10u8), 10u8);
        }

        #[test]
        fn check_question_count() {
            let question = Question {
                qname: vec![],
                qtype: QType::RType(RType::NS),
                qclass: QClass::RClass(RClass::IN),
            };
            let query = QueryBuilder::new().questions(vec![question]).build();
            let mut buf = Vec::<u8>::new();
            let result = query.to_wire(&mut buf, 512);

            assert!(result.is_ok());
            assert_eq!(buf[4], 0u8);
            assert_eq!(buf[5], 1u8);
        }

        #[test]
        fn check_question_count_large() {
            let question = Question {
                qname: vec![],
                qtype: QType::RType(RType::NS),
                qclass: QClass::RClass(RClass::IN),
            };
            let query = QueryBuilder::new()
                .questions(vec![question; 0x01AB])
                .build();
            let mut buf = Vec::<u8>::new();
            let result = query.to_wire(&mut buf, 12 + 0x01AB * 5);

            assert!(result.is_ok());
            assert_eq!(buf[4], 0x01u8);
            assert_eq!(buf[5], 0xABu8);
        }

        #[test]
        fn check_buf_offset() {
            let query = QueryBuilder::new().id(0xCCDDu16).build();
            let mut buf = vec![0xAAu8, 0xBBu8];
            let result = query.to_wire(&mut buf, 512);

            assert!(result.is_ok());
            assert_eq!(buf.len(), 14);
            assert_eq!(buf[0], 0xAAu8);
            assert_eq!(buf[1], 0xBBu8);
            assert_eq!(buf[2], 0xCCu8);
            assert_eq!(buf[3], 0xDDu8);
        }
    }

    mod question {
        use super::*;

        #[test]
        fn check_question() {
            let question = Question {
                qname: vec![b"example".to_vec(), b"com".to_vec()],
                qtype: QType::RType(RType::NS),
                qclass: QClass::RClass(RClass::IN),
            };
            let query = QueryBuilder::new().questions(vec![question]).build();
            let mut buf = Vec::<u8>::new();
            let result = query.to_wire(&mut buf, 512);

            assert!(result.is_ok());
            assert_eq!(buf.len(), 29);
            assert_eq!(buf[12], 7u8);
            assert_eq!(buf[13..20], *b"example");
            assert_eq!(buf[20], 3u8);
            assert_eq!(buf[21..24], *b"com");
            assert_eq!(buf[24], 0u8);

            // NS type -> 2
            assert_eq!(buf[25], 0u8);
            assert_eq!(buf[26], 2u8);

            // IN class -> 1
            assert_eq!(buf[27], 0u8);
            assert_eq!(buf[28], 1u8);
        }

        #[test]
        fn check_qtype_big_endian() {
            let question = Question {
                qname: vec![b"example".to_vec(), b"com".to_vec()],
                qtype: QType::RType(RType::Unknown(0xABCDu16)),
                qclass: QClass::RClass(RClass::IN),
            };
            let query = QueryBuilder::new().questions(vec![question]).build();
            let mut buf = Vec::<u8>::new();
            let result = query.to_wire(&mut buf, 512);

            assert!(result.is_ok());
            assert_eq!(buf[25], 0xABu8);
            assert_eq!(buf[26], 0xCDu8);
        }

        #[test]
        fn check_qclass_big_endian() {
            let question = Question {
                qname: vec![b"example".to_vec(), b"com".to_vec()],
                qtype: QType::RType(RType::A),
                qclass: QClass::RClass(RClass::Unknown(0xABCD)),
            };
            let query = QueryBuilder::new().questions(vec![question]).build();
            let mut buf = Vec::<u8>::new();
            let result = query.to_wire(&mut buf, 512);

            assert!(result.is_ok());
            assert_eq!(buf[27], 0xABu8);
            assert_eq!(buf[28], 0xCDu8);
        }

        #[test]
        fn check_question_root() {
            let question = Question {
                qname: vec![],
                qtype: QType::RType(RType::NS),
                qclass: QClass::RClass(RClass::IN),
            };
            let query = QueryBuilder::new().questions(vec![question]).build();
            let mut buf = Vec::<u8>::new();
            let result = query.to_wire(&mut buf, 512);

            assert!(result.is_ok());
            //assert_eq!(buf.len(), 17);
            assert_eq!(buf[12], 0u8);

            // NS type -> 2
            assert_eq!(buf[13], 0u8);
            assert_eq!(buf[14], 2u8);

            // IN class -> 1
            assert_eq!(buf[15], 0u8);
            assert_eq!(buf[16], 1u8);
        }

        #[test]
        fn check_multiple_question() {
            let question1 = Question {
                qname: vec![b"example".to_vec(), b"com".to_vec()],
                qtype: QType::RType(RType::A),
                qclass: QClass::RClass(RClass::CS),
            };
            let question2 = Question {
                qname: vec![b"domain".to_vec(), b"org".to_vec()],
                qtype: QType::RType(RType::MX),
                qclass: QClass::RClass(RClass::HS),
            };

            let query = QueryBuilder::new()
                .questions(vec![question1, question2])
                .build();
            let mut buf = Vec::<u8>::new();
            let result = query.to_wire(&mut buf, 512);

            assert!(result.is_ok());
            assert_eq!(buf.len(), 45);

            // Question 1
            assert_eq!(buf[12], 7u8);
            assert_eq!(buf[13..20], *b"example");
            assert_eq!(buf[20], 3u8);
            assert_eq!(buf[21..24], *b"com");
            assert_eq!(buf[24], 0u8);

            // A type -> 1
            assert_eq!(buf[25], 0u8);
            assert_eq!(buf[26], 1u8);

            // CS class -> 3
            assert_eq!(buf[27], 0u8);
            assert_eq!(buf[28], 2u8);

            // Question 2
            assert_eq!(buf[29], 6u8);
            assert_eq!(buf[30..36], *b"domain");
            assert_eq!(buf[36], 3u8);
            assert_eq!(buf[37..40], *b"org");
            assert_eq!(buf[40], 0u8);

            // MX type -> 15
            assert_eq!(buf[41], 0u8);
            assert_eq!(buf[42], 15u8);

            // HS class -> 4
            assert_eq!(buf[43], 0u8);
            assert_eq!(buf[44], 4u8);
        }

        #[test]
        fn check_label_not_too_long() {
            let label = vec![b'a'; 63];
            let question = Question {
                qname: vec![label.clone()],
                qtype: QType::RType(RType::NS),
                qclass: QClass::RClass(RClass::IN),
            };
            let query = QueryBuilder::new().questions(vec![question]).build();
            let mut buf = Vec::<u8>::new();
            let result = query.to_wire(&mut buf, 512);

            assert!(result.is_ok());
            assert_eq!(buf.len(), 81);
            assert_eq!(buf[12], 63u8);
            assert_eq!(buf[13..76], label);
            assert_eq!(buf[76], 0u8);
        }
    }

    mod truncation {
        use super::*;

        #[test]
        fn check_no_truncation() {
            let query = QueryBuilder::new().build();
            let mut buf = Vec::<u8>::new();
            let result = query.to_wire(&mut buf, 512);

            assert!(result.is_ok());
            assert!(!result.unwrap().truncation);
            assert_eq!(buf[2] & 0b0000_0010u8, 0b0000_0000u8); // TC flag unset
            assert_eq!(buf.len(), 12);
        }

        #[test]
        fn check_no_truncation_one_question() {
            let question = Question {
                qname: vec![],
                qtype: QType::RType(RType::NS),
                qclass: QClass::RClass(RClass::IN),
            };
            let query = QueryBuilder::new().questions(vec![question]).build();
            let mut buf = Vec::<u8>::new();
            let result = query.to_wire(&mut buf, 512);

            assert!(result.is_ok());
            assert!(!result.unwrap().truncation);
            assert_eq!(buf[2] & 0b0000_0010u8, 0b0000_0000u8); // TC flag unset
            assert_eq!(buf.len(), 17);

            // Question count
            assert_eq!(buf[4], 0u8);
            assert_eq!(buf[5], 1u8);
        }

        #[test]
        fn check_truncation_one_question() {
            let question = Question {
                qname: vec![],
                qtype: QType::RType(RType::NS),
                qclass: QClass::RClass(RClass::IN),
            };
            let query = QueryBuilder::new().questions(vec![question]).build();
            let mut buf = Vec::<u8>::new();
            let result = query.to_wire(&mut buf, 12);

            assert!(result.is_ok());
            assert!(result.unwrap().truncation);
            assert_eq!(buf[2] & 0b0000_0010u8, 0b0000_0010u8); // TC flag set
            assert_eq!(buf.len(), 12);

            // Question count
            assert_eq!(buf[4], 0u8);
            assert_eq!(buf[5], 0u8);
        }

        #[test]
        fn check_truncation_more_questions() {
            let question = Question {
                qname: vec![],
                qtype: QType::RType(RType::NS),
                qclass: QClass::RClass(RClass::IN),
            };
            let query = QueryBuilder::new().questions(vec![question; 3]).build();
            let mut buf = Vec::<u8>::new();
            let result = query.to_wire(&mut buf, 22);

            assert!(result.is_ok());
            assert!(result.unwrap().truncation);
            assert_eq!(buf[2] & 0b0000_0010u8, 0b0000_0010u8); // TC flag set
            assert_eq!(buf.len(), 22);

            // Question count
            assert_eq!(buf[4], 0u8);
            assert_eq!(buf[5], 2u8);
        }

        #[test]
        fn check_no_truncation_buf_offset() {
            let query = QueryBuilder::new().build();
            let mut buf = vec![0xAAu8, 0xBBu8];
            let result = query.to_wire(&mut buf, 12);

            assert!(result.is_ok());
            assert!(!result.unwrap().truncation);
            assert_eq!(buf.len(), 14);
            assert_eq!(buf[4] & 0b0000_0010u8, 0b0000_0000u8); // TC flag unset
        }

        #[test]
        fn check_truncation_buf_offset() {
            let question = Question {
                qname: vec![],
                qtype: QType::RType(RType::NS),
                qclass: QClass::RClass(RClass::IN),
            };
            let query = QueryBuilder::new().questions(vec![question]).build();
            let mut buf = vec![0xAAu8, 0xBBu8];
            let result = query.to_wire(&mut buf, 12);

            assert!(result.is_ok());
            assert!(result.unwrap().truncation);
            assert_eq!(buf[4] & 0b0000_0010u8, 0b0000_0010u8); // TC flag set
            assert_eq!(buf.len(), 14);

            // Question count
            assert_eq!(buf[6], 0u8);
            assert_eq!(buf[7], 0u8);
        }
    }
}
