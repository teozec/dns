use std::net::Ipv4Addr;

use crate::types::{Opcode, QClass, QType, RClass, RType, ResponseCode};

pub type DomainName = Vec<Vec<u8>>;

#[derive(Debug)]
pub struct Message {
    pub id: u16,
    pub kind: MessageKind,
    pub opcode: Opcode,
    pub recursion_desired: bool,
    pub questions: Vec<Question>,
}

#[derive(Debug)]
pub enum MessageKind {
    Query,
    Response {
        authoritative: bool,
        recursion_available: bool,
        response_code: ResponseCode,
        answer: Vec<ResourceRecord>,
        authority: Vec<ResourceRecord>,
        additional: Vec<ResourceRecord>,
    },
}

#[derive(Clone, Debug)]
pub struct Question {
    pub qname: DomainName,
    pub qtype: QType,
    pub qclass: QClass,
}

#[derive(Debug, Clone)]
pub struct ResourceRecord {
    pub name: DomainName,
    pub class: RClass,
    pub ttl: u32,
    pub rdata: ResourceRecordData,
}

#[derive(Debug, Clone)]
pub enum ResourceRecordData {
    A(Ipv4Addr),
    NS(DomainName),
    MD(DomainName),
    MF(DomainName),
    CNAME(DomainName),
    SOA {
        mname: DomainName,
        rname: DomainName,
        serial: u32,
        refresh: u32,
        retry: u32,
        expire: u32,
        minimum: u32,
    },
    MB(DomainName),
    MG(DomainName),
    MR(DomainName),
    NULL(Vec<u8>),
    WKS {
        address: Ipv4Addr,
        protocol: u8,
        bitmap: Vec<u8>,
    },
    PTR(DomainName),
    HINFO {
        cpu: Vec<u8>,
        os: Vec<u8>,
    },
    MINFO {
        rmailbx: DomainName,
        emailbx: DomainName,
    },
    MX {
        preference: u16,
        exchange: DomainName,
    },
    TXT(Vec<Vec<u8>>),
    Unknown {
        rtype: u16,
        rdata: Vec<u8>,
    },
}

impl From<&ResourceRecordData> for RType {
    fn from(record: &ResourceRecordData) -> RType {
        match record {
            ResourceRecordData::A(_) => RType::A,
            ResourceRecordData::NS(_) => RType::NS,
            ResourceRecordData::MD(_) => RType::MD,
            ResourceRecordData::MF(_) => RType::MF,
            ResourceRecordData::CNAME(_) => RType::CNAME,
            ResourceRecordData::SOA { .. } => RType::SOA,
            ResourceRecordData::MB(_) => RType::MB,
            ResourceRecordData::MG(_) => RType::MG,
            ResourceRecordData::MR(_) => RType::MR,
            ResourceRecordData::NULL(_) => RType::NULL,
            ResourceRecordData::WKS { .. } => RType::WKS,
            ResourceRecordData::PTR(_) => RType::PTR,
            ResourceRecordData::HINFO { .. } => RType::HINFO,
            ResourceRecordData::MINFO { .. } => RType::MINFO,
            ResourceRecordData::MX { .. } => RType::MX,
            ResourceRecordData::TXT(_) => RType::TXT,
            ResourceRecordData::Unknown { rtype, .. } => RType::Unknown(*rtype),
        }
    }
}
