#[derive(Debug, Clone, Copy)]
pub enum Opcode {
    Standard,
    Inverse,
    Status,
    Unknown(u8),
}

#[derive(Debug, Clone, Copy)]
pub enum ResponseCode {
    NoError,
    FormatError,
    ServerFailure,
    NameError,
    NotImplemented,
    Refused,
    Unknown(u8),
}

#[derive(Debug, Clone, Copy)]
pub enum RType {
    A,
    NS,
    MD,
    MF,
    CNAME,
    SOA,
    MB,
    MG,
    MR,
    NULL,
    WKS,
    PTR,
    HINFO,
    MINFO,
    MX,
    TXT,
    Unknown(u16),
}

#[derive(Debug, Clone, Copy)]
pub enum QType {
    RType(RType),
    AXFR,
    MAILB,
    MAILA,
    Any,
}

#[derive(Debug, Clone, Copy)]
pub enum RClass {
    IN,
    CS,
    CH,
    HS,
    Unknown(u16),
}

#[derive(Debug, Clone, Copy)]
pub enum QClass {
    RClass(RClass),
    Any,
}
