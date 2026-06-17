//! This module defines vocabulary types that are used in DNS messages.

#[derive(Debug, Clone, Copy)]
pub enum Opcode {
    /// RFC 1034 - Section 3.7.1
    /// A standard query specifies a target domain name (QNAME), query type
    /// (QTYPE), and query class (QCLASS) and asks for RRs which match.    Standard,
    Standard,

    /// RFC 1034 - Section 3.7.2
    /// Name servers may also support inverse queries that map a particular
    /// resource to a domain name or domain names that have that resource. [...]
    /// Implementation of this service is optional in a name server, but all
    /// name servers must at least be able to understand an inverse query
    /// message and return a not-implemented error response.
    Inverse,

    /// RFC 1034 - Section 3.8
    /// To be defined.
    Status,

    Unknown(u8),
}

/// RFC 1035 - Section 4.1.1
#[derive(Debug, Clone, Copy)]
pub enum ResponseCode {
    /// No error condition
    NoError,

    /// Format error
    /// The name server was unable to interpret the query.
    FormatError,

    /// Server failure
    /// The name server was unable to process this query due to a problem with the name server.
    ServerFailure,

    /// Name Error
    /// Meaningful only for responses from an authoritative name server, this code signifies
    /// that the domain name referenced in the query does not exist.
    NameError,

    /// Not Implemented
    /// The name server does not support the requested kind of query.
    NotImplemented,

    /// Refused
    /// The name server refuses to perform the specified operation for policy reasons.
    Refused,

    Unknown(u8),
}

/// RFC 1035 - Section 3.2.2
#[derive(Debug, Clone, Copy)]
pub enum RType {
    /// A host address
    A,

    /// An authoritative name server
    NS,

    /// A mail destination (Obsolete - use MX)
    MD,

    /// A mail forwarder (Obsolete - use MX)
    MF,

    /// The canonical name for an alias
    CNAME,

    /// Marks the start of a zone of authority
    SOA,

    /// A mailbox domain name (EXPERIMENTAL)
    MB,

    /// A mail group member (EXPERIMENTAL)
    MG,

    /// A mail rename domain name (EXPERIMENTAL)
    MR,

    /// A null RR (EXPERIMENTAL)
    NULL,

    /// A well known service description
    WKS,

    /// A domain name pointer
    PTR,

    /// Host information
    HINFO,

    /// Mailbox or mail list information
    MINFO,

    /// Mail exchange
    MX,

    /// Text strings
    TXT,

    Unknown(u16),
}

/// RFC 1035 - Section 3.2.3
#[derive(Debug, Clone, Copy)]
pub enum QType {
    RType(RType),

    /// A request for a transfer of an entire zone
    AXFR,

    /// A request for mailbox-related records (MB, MG or MR)
    MAILB,

    /// A request for mail agent RRs (Obsolete - see MX)
    MAILA,

    /// A request for all records
    Any,
}

/// RFC 1035 - Section 3.2.4
#[derive(Debug, Clone, Copy)]
pub enum RClass {
    /// The Internet
    IN,

    /// The CSNET class (Obsolete)
    CS,

    /// The CHAOS class
    CH,

    /// Hesiod
    HS,

    Unknown(u16),
}

/// RFC 1035 - Section 3.2.5
#[derive(Debug, Clone, Copy)]
pub enum QClass {
    RClass(RClass),

    /// Any class
    Any,
}
