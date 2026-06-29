use std::net::Ipv4Addr;

use crate::{
    message::{
        CharacterString, DomainName, Message, MessageKind, Question, ResourceRecord,
        ResourceRecordData,
    },
    types::{Opcode, QClass, QType, RClass, RType, ResponseCode},
};

pub trait FromWire {
    fn from_wire(buf: &[u8]) -> Result<FromWireResult, FromWireError>;
}

#[derive(Debug)]
pub struct FromWireResult {
    pub message: Message,
    pub truncation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FromWireError {
    BufferTooShort,
    InvalidHeader,
    InvalidQuestion,
    InvalidResourceRecord,
    InvalidDomainName,
    InvalidDomainNamePointer,
    InvalidCharacterString,
    LabelTooLong,
    DomainNameTooLong,
    TrailingBytes,
}

impl FromWire for Message {
    fn from_wire(buf: &[u8]) -> Result<FromWireResult, FromWireError> {
        if buf.len() < 12 {
            return Err(FromWireError::BufferTooShort);
        }

        let mut parser = Parser::new(buf);
        let header = Header::from_wire_entity(&mut parser)?;

        let question = (0..header.qdcount)
            .map(|_| Question::from_wire_entity(&mut parser))
            .collect::<Result<Vec<_>, _>>()?;

        let answer = (0..header.ancount)
            .map(|_| ResourceRecord::from_wire_entity(&mut parser))
            .collect::<Result<Vec<_>, _>>()?;

        let authority = (0..header.nscount)
            .map(|_| ResourceRecord::from_wire_entity(&mut parser))
            .collect::<Result<Vec<_>, _>>()?;

        let additional = (0..header.arcount)
            .map(|_| ResourceRecord::from_wire_entity(&mut parser))
            .collect::<Result<Vec<_>, _>>()?;

        if parser.pos != buf.len() {
            return Err(FromWireError::TrailingBytes);
        }

        let message = Message {
            id: header.id,
            kind: header.kind,
            opcode: header.opcode,
            recursion_desired: header.recursion_desired,
            question,
            answer,
            authority,
            additional,
        };

        Ok(FromWireResult {
            message,
            truncation: header.truncation,
        })
    }
}

struct Parser<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl Parser<'_> {
    fn new(buf: &[u8]) -> Parser<'_> {
        Parser { buf, pos: 0usize }
    }

    fn at(&self, pos: usize) -> Parser<'_> {
        Parser { buf: self.buf, pos }
    }

    fn read_array<const N: usize>(&mut self) -> Option<[u8; N]> {
        let bytes = self.buf.get(self.pos..self.pos + N)?.try_into().unwrap(); // safe: slice is exactly N bytes

        self.pos += N;
        Some(bytes)
    }

    fn read_u8(&mut self) -> Option<u8> {
        self.read_array().map(|a: [u8; 1]| a[0])
    }

    fn read_u16(&mut self) -> Option<u16> {
        self.read_array().map(u16::from_be_bytes)
    }

    fn read_u32(&mut self) -> Option<u32> {
        self.read_array().map(u32::from_be_bytes)
    }

    fn read_bytes(&mut self, n: usize) -> Option<Vec<u8>> {
        let res = self.buf.get(self.pos..self.pos + n).map(Vec::from)?;

        self.pos += n;
        Some(res)
    }
}

struct Header {
    id: u16,
    kind: MessageKind,
    opcode: Opcode,
    truncation: bool,
    recursion_desired: bool,
    qdcount: u16,
    ancount: u16,
    nscount: u16,
    arcount: u16,
}

trait FromWireEntity: Sized {
    fn from_wire_entity(parser: &mut Parser) -> Result<Self, FromWireError>;
}

impl FromWireEntity for Header {
    fn from_wire_entity(parser: &mut Parser) -> Result<Self, FromWireError> {
        let id = parser.read_u16().ok_or(FromWireError::InvalidHeader)?;
        let header_info = parser.read_u16().ok_or(FromWireError::InvalidHeader)?;
        let is_query = header_info & (1u16 << 15) == 0u16;
        let opcode = Opcode::from((header_info >> 11) & 0b1111u16);
        let authoritative = header_info & (1u16 << 10) != 0u16;
        let truncation = header_info & (1u16 << 9) != 0u16;
        let recursion_desired = header_info & (1u16 << 8) != 0u16;
        let recursion_available = header_info & (1u16 << 7) != 0u16;
        let rcode = header_info & 0b1111u16;

        if is_query && (authoritative || recursion_available || rcode != 0u16) {
            return Err(FromWireError::InvalidHeader);
        }

        let response_code = ResponseCode::from(rcode);
        let kind = if is_query {
            MessageKind::Query
        } else {
            MessageKind::Response {
                authoritative,
                recursion_available,
                response_code,
            }
        };

        let qdcount = parser.read_u16().ok_or(FromWireError::InvalidHeader)?;
        let ancount = parser.read_u16().ok_or(FromWireError::InvalidHeader)?;
        let nscount = parser.read_u16().ok_or(FromWireError::InvalidHeader)?;
        let arcount = parser.read_u16().ok_or(FromWireError::InvalidHeader)?;

        Ok(Header {
            id,
            kind,
            opcode,
            truncation,
            recursion_desired,
            qdcount,
            ancount,
            nscount,
            arcount,
        })
    }
}

impl FromWireEntity for Question {
    /// RFC 1035 - Section 4.1.2
    fn from_wire_entity(parser: &mut Parser) -> Result<Self, FromWireError> {
        let qname = DomainName::from_wire_entity(parser)?;
        let qtype = QType::from(parser.read_u16().ok_or(FromWireError::InvalidQuestion)?);
        let qclass = QClass::from(parser.read_u16().ok_or(FromWireError::InvalidQuestion)?);
        Ok(Question {
            qname,
            qtype,
            qclass,
        })
    }
}

impl FromWireEntity for ResourceRecord {
    fn from_wire_entity(parser: &mut Parser) -> Result<Self, FromWireError> {
        let name = DomainName::from_wire_entity(parser)?;
        let rtype = RType::from(
            parser
                .read_u16()
                .ok_or(FromWireError::InvalidResourceRecord)?,
        );
        let class = RClass::from(
            parser
                .read_u16()
                .ok_or(FromWireError::InvalidResourceRecord)?,
        );

        let ttl = parser
            .read_u32()
            .ok_or(FromWireError::InvalidResourceRecord)?;

        let rdlength = parser
            .read_u16()
            .ok_or(FromWireError::InvalidResourceRecord)?;

        let rdata_start = parser.pos;
        let rdata = parse_rdata(parser, rtype, class, rdlength)?;

        if parser.pos != rdata_start + rdlength as usize {
            Err(FromWireError::InvalidResourceRecord)
        } else {
            Ok(ResourceRecord {
                name,
                class,
                ttl,
                rdata,
            })
        }
    }
}

fn parse_rdata(
    parser: &mut Parser,
    rtype: RType,
    rclass: RClass,
    rdlength: u16,
) -> Result<ResourceRecordData, FromWireError> {
    Ok(match rtype {
        RType::A if let RClass::IN = rclass => ResourceRecordData::A(Ipv4Addr::from_octets(
            parser
                .read_array()
                .ok_or(FromWireError::InvalidResourceRecord)?,
        )),
        RType::A => ResourceRecordData::Unknown {
            rtype: RType::A.into(),
            rdata: parser
                .read_bytes(rdlength as usize)
                .ok_or(FromWireError::InvalidResourceRecord)?,
        },
        RType::NS => ResourceRecordData::NS(DomainName::from_wire_entity(parser)?),
        RType::MD => ResourceRecordData::MD(DomainName::from_wire_entity(parser)?),
        RType::MF => ResourceRecordData::MF(DomainName::from_wire_entity(parser)?),
        RType::CNAME => ResourceRecordData::CNAME(DomainName::from_wire_entity(parser)?),
        RType::SOA => ResourceRecordData::SOA {
            mname: DomainName::from_wire_entity(parser)?,
            rname: DomainName::from_wire_entity(parser)?,
            serial: parser
                .read_u32()
                .ok_or(FromWireError::InvalidResourceRecord)?,
            refresh: parser
                .read_u32()
                .ok_or(FromWireError::InvalidResourceRecord)?,
            retry: parser
                .read_u32()
                .ok_or(FromWireError::InvalidResourceRecord)?,
            expire: parser
                .read_u32()
                .ok_or(FromWireError::InvalidResourceRecord)?,
            minimum: parser
                .read_u32()
                .ok_or(FromWireError::InvalidResourceRecord)?,
        },
        RType::MB => ResourceRecordData::MB(DomainName::from_wire_entity(parser)?),
        RType::MG => ResourceRecordData::MG(DomainName::from_wire_entity(parser)?),
        RType::MR => ResourceRecordData::MR(DomainName::from_wire_entity(parser)?),
        RType::NULL => ResourceRecordData::NULL(
            parser
                .read_bytes(rdlength as usize)
                .ok_or(FromWireError::InvalidResourceRecord)?,
        ),
        RType::WKS if let RClass::IN = rclass => ResourceRecordData::WKS {
            address: Ipv4Addr::from_octets(
                parser
                    .read_array()
                    .ok_or(FromWireError::InvalidResourceRecord)?,
            ),
            protocol: parser
                .read_u8()
                .ok_or(FromWireError::InvalidResourceRecord)?,
            bitmap: parser
                .read_bytes(
                    // address (4) + protocol (1) already consumed; guard against
                    // a declared RDLENGTH smaller than that fixed prefix.
                    (rdlength as usize)
                        .checked_sub(5)
                        .ok_or(FromWireError::InvalidResourceRecord)?,
                )
                .ok_or(FromWireError::InvalidResourceRecord)?,
        },
        RType::WKS => ResourceRecordData::Unknown {
            rtype: RType::WKS.into(),
            rdata: parser
                .read_bytes(rdlength as usize)
                .ok_or(FromWireError::InvalidResourceRecord)?,
        },
        RType::PTR => ResourceRecordData::PTR(DomainName::from_wire_entity(parser)?),
        RType::HINFO => ResourceRecordData::HINFO {
            cpu: CharacterString::from_wire_entity(parser)?,
            os: CharacterString::from_wire_entity(parser)?,
        },
        RType::MINFO => ResourceRecordData::MINFO {
            rmailbx: DomainName::from_wire_entity(parser)?,
            emailbx: DomainName::from_wire_entity(parser)?,
        },
        RType::MX => ResourceRecordData::MX {
            preference: parser
                .read_u16()
                .ok_or(FromWireError::InvalidResourceRecord)?,
            exchange: DomainName::from_wire_entity(parser)?,
        },
        RType::TXT => {
            let mut strings = vec![];
            let mut remaining = rdlength as usize;
            while remaining > 0 {
                let string = CharacterString::from_wire_entity(parser)?;
                // checked_sub consumes the string plus its length octet and, in the
                // same step, rejects a <character-string> that overruns the RDATA.
                remaining = remaining
                    .checked_sub(string.len() + 1)
                    .ok_or(FromWireError::InvalidResourceRecord)?;
                strings.push(string);
            }
            // RFC 1035 - Section 3.3.14: TXT RDATA is "one or more" <character-string>s.
            if strings.is_empty() {
                return Err(FromWireError::InvalidResourceRecord);
            }
            ResourceRecordData::TXT(strings)
        }
        RType::Unknown(rtype) => ResourceRecordData::Unknown {
            rtype,
            rdata: parser
                .read_bytes(rdlength as usize)
                .ok_or(FromWireError::InvalidResourceRecord)?,
        },
    })
}

impl FromWireEntity for DomainName {
    fn from_wire_entity(parser: &mut Parser) -> Result<Self, FromWireError> {
        let mut domain_name = vec![];
        parse_domain_name(parser, &mut domain_name, 0usize, 20usize)?;
        Ok(domain_name)
    }
}

fn parse_domain_name(
    parser: &mut Parser,
    name: &mut DomainName,
    mut total_len: usize,
    recursion_limit: usize,
) -> Result<(), FromWireError> {
    loop {
        let pos = parser.pos;
        let len = parser.read_u8().ok_or(FromWireError::InvalidDomainName)?;

        if recursion_limit == 0usize {
            return Err(FromWireError::InvalidDomainName);
        } else if len & 0b1100_0000u8 == 0b1100_0000u8 {
            let second = parser.read_u8().ok_or(FromWireError::InvalidDomainName)?;
            let offset = u16::from_be_bytes([len & 0b0011_1111u8, second]) as usize;
            if offset >= pos {
                return Err(FromWireError::InvalidDomainNamePointer);
            } else {
                return parse_domain_name(
                    &mut parser.at(offset),
                    name,
                    total_len,
                    recursion_limit - 1,
                );
            }
        } else if len == 0u8 {
            return Ok(());
        } else if len >= 64u8 {
            return Err(FromWireError::LabelTooLong);
        } else {
            let label = parser
                .read_bytes(len.into())
                .ok_or(FromWireError::InvalidDomainName)?;

            total_len = total_len + label.len() + 1usize;

            // RFC 1035 - Section 3.1: the total length of a domain name (label
            // octets plus the length octet of each label, including the final
            // zero-length root label) is restricted to 255 octets or less.
            //
            // We add 1 to account for the final 0-length octet.
            if total_len + 1 > 255 {
                return Err(FromWireError::DomainNameTooLong);
            } else {
                name.push(label);
            }
        }
    }
}

impl FromWireEntity for CharacterString {
    fn from_wire_entity(parser: &mut Parser) -> Result<Self, FromWireError> {
        let len = parser.read_u8().ok_or(FromWireError::InvalidDomainName)?;

        parser
            .read_bytes(len.into())
            .ok_or(FromWireError::InvalidCharacterString)
    }
}

/// RFC 1035 - Section 4.1.1
impl From<u8> for Opcode {
    fn from(opcode: u8) -> Opcode {
        match opcode {
            0u8 => Opcode::Standard,
            1u8 => Opcode::Inverse,
            2u8 => Opcode::Status,
            o => Opcode::Unknown(o),
        }
    }
}

impl From<u16> for Opcode {
    fn from(opcode: u16) -> Opcode {
        Opcode::from(opcode as u8)
    }
}

/// RFC 1035 - Section 4.1.1
impl From<u8> for ResponseCode {
    fn from(response_code: u8) -> ResponseCode {
        match response_code {
            0u8 => ResponseCode::NoError,
            1u8 => ResponseCode::FormatError,
            2u8 => ResponseCode::ServerFailure,
            3u8 => ResponseCode::NameError,
            4u8 => ResponseCode::NotImplemented,
            5u8 => ResponseCode::Refused,
            c => ResponseCode::Unknown(c),
        }
    }
}

impl From<u16> for ResponseCode {
    fn from(response_code: u16) -> ResponseCode {
        ResponseCode::from(response_code as u8)
    }
}

/// RFC 1035 - Section 3.2.4
impl From<u16> for RClass {
    fn from(class: u16) -> RClass {
        match class {
            1u16 => RClass::IN,
            2u16 => RClass::CS,
            3u16 => RClass::CH,
            4u16 => RClass::HS,
            c => RClass::Unknown(c),
        }
    }
}

/// RFC 1035 - Section 3.2.5
impl From<u16> for QClass {
    fn from(qclass: u16) -> QClass {
        match qclass {
            255u16 => QClass::Any,
            c => QClass::RClass(c.into()),
        }
    }
}

/// RFC 1035 - Section 3.2.2
impl From<u16> for RType {
    fn from(qtype: u16) -> RType {
        match qtype {
            1u16 => RType::A,
            2u16 => RType::NS,
            3u16 => RType::MD,
            4u16 => RType::MF,
            5u16 => RType::CNAME,
            6u16 => RType::SOA,
            7u16 => RType::MB,
            8u16 => RType::MG,
            9u16 => RType::MR,
            10u16 => RType::NULL,
            11u16 => RType::WKS,
            12u16 => RType::PTR,
            13u16 => RType::HINFO,
            14u16 => RType::MINFO,
            15u16 => RType::MX,
            16u16 => RType::TXT,
            t => RType::Unknown(t),
        }
    }
}

/// RFC 1035 - Section 3.2.3
impl From<u16> for QType {
    fn from(qtype: u16) -> QType {
        match qtype {
            252u16 => QType::AXFR,
            253u16 => QType::MAILB,
            254u16 => QType::MAILA,
            255u16 => QType::Any,
            t => QType::RType(t.into()),
        }
    }
}

#[cfg(test)]
mod tests {}
