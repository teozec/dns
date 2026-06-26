use std::net::Ipv4Addr;

use crate::types::{Opcode, QClass, QType, RClass, RType, ResponseCode};

/// RFC 1034 - Section 3.1
///
/// The domain name space is a tree structure.  Each node and leaf on the
/// tree corresponds to a resource set (which may be empty).  [...]
///
/// Each node has a label, which is zero to 63 octets in length.  Brother
/// nodes may not have the same label, although the same label can be used
/// for nodes which are not brothers.  One label is reserved, and that is
/// the null (i.e., zero length) label used for the root.
///
/// The domain name of a node is the list of the labels on the path from the
/// node to the root of the tree.  By convention, the labels that compose a
/// domain name are printed or read left to right, from the most specific
/// (lowest, farthest from the root) to the least specific (highest, closest
/// to the root).
///
/// RFC 1035 - Section 3.1.
///
/// Domain names in messages are expressed in terms of a sequence of labels.
/// Each label is represented as a one octet length field followed by that
/// number of octets.  Since every domain name ends with the null label of
/// the root, a domain name is terminated by a length byte of zero.  The
/// high order two bits of every length octet must be zero, and the
/// remaining six bits of the length field limit the label to 63 octets or
/// less.
///
/// To simplify implementations, the total length of a domain name (i.e.,
/// label octets and label length octets) is restricted to 255 octets or
/// less.
pub type DomainName = Vec<Vec<u8>>;

/// RFC 1035 - Section 3.3
/// <character-string> is a single length octet followed by that number of characters.
/// <character-string> is treated as binary information, and can be up to 256 characters
/// in length (including the length octet).
pub type CharacterString = Vec<u8>;

/// RFC 1034 - Section 4.3.1
///
/// The principal activity of name servers is to answer standard queries.
/// Both the query and its response are carried in a standard message format
/// which is described in RFC 1035.
///
/// RFC 1035 - Section 4.1
///
/// All communications inside of the domain protocol are carried in a single
/// format called a message.  The top level format of message is divided
/// into 5 sections (some of which are empty in certain cases) .
///
///  The header section is always present.  The header includes fields that
/// specify which of the remaining sections are present, and also specify
/// whether the message is a query or a response, a standard query or some
/// other opcode, etc.
///
/// The names of the sections after the header are derived from their use in
/// standard queries.  The question section contains fields that describe a
/// question to a name server.  These fields are a query type (QTYPE), a
/// query class (QCLASS), and a query domain name (QNAME).  The last three
/// sections have the same format: a possibly empty list of concatenated
/// resource records (RRs).  The answer section contains RRs that answer the
/// question; the authority section contains RRs that point toward an
/// authoritative name server; the additional records section contains RRs
/// which relate to the query, but are not strictly answers for the
/// question.
#[derive(Debug)]
pub struct Message {
    /// A 16 bit identifier assigned by the program that generates any kind of query.
    /// This identifier is copied in the corresponding reply and can be used by the requester
    /// to match up replies to outstanding queries.
    pub id: u16,

    /// A one bit field that specifies whether this message is a query (0), or a response (1).
    /// Includes also the data specific for queries and responses.
    pub kind: MessageKind,

    /// A four bit field that specifies kind of query in this message.
    /// This value is set by the originator of a query and copied into the response.
    pub opcode: Opcode,

    /// This bit may be set in a query and is copied into the response.
    /// If RD is set, it directs the name server to pursue the query recursively.
    /// Recursive query support is optional.
    pub recursion_desired: bool,

    /// The question section is used to carry the "question" in most queries,
    /// i.e., the parameters that define what is being asked.
    pub question: Vec<Question>,

    /// The answer section contains RRs that answer the question
    pub answer: Vec<ResourceRecord>,

    /// The authority section contains RRs that point toward an authoritative name server
    pub authority: Vec<ResourceRecord>,

    /// The additional records section contains RRs which relate to the query, but are not strictly answers for the question
    pub additional: Vec<ResourceRecord>,
}

#[derive(Debug)]
pub enum MessageKind {
    Query,
    Response {
        /// This bit is valid in responses, and specifies that the responding name server
        /// is an authority for the domain name in question section.
        authoritative: bool,

        /// This bit is set or cleared in a response, and denotes whether recursive query support
        /// is available in the name server.
        recursion_available: bool,

        /// This 4 bit field is set as part of responses.
        response_code: ResponseCode,
    },
}

/// RFC 1034 - Section 3.7
///
/// Carries the query name and other query parameters.
///
/// RFC 1035 - Section 4.1.2
///
/// The question section is used to carry the "question" in most queries,
/// i.e., the parameters that define what is being asked.  The section
/// contains QDCOUNT (usually 1) entries,
#[derive(Clone, Debug)]
pub struct Question {
    /// A domain name represented as a sequence of labels
    pub qname: DomainName,

    /// A [...] code which specifies the type of the query.
    /// The values for this field include all codes valid for a
    /// TYPE field, together with some more general codes which
    /// can match more than one type of RR.
    pub qtype: QType,

    /// A [...] code that specifies the class of the query.
    /// For example, the QCLASS field is IN for the Internet.
    pub qclass: QClass,
}

/// RFC 1034 - Section 3.6
///
/// A domain name identifies a node.  Each node has a set of resource
/// information, which may be empty.  The set of resource information
/// associated with a particular name is composed of separate resource
/// records (RRs).  The order of RRs in a set is not significant, and need
/// not be preserved by name servers, resolvers, or other parts of the DNS.
#[derive(Debug, Clone)]
pub struct ResourceRecord {
    /// The domain name where the RR is found.
    pub name: DomainName,

    /// Identifies a protocol family or instance of a protocol.
    pub class: RClass,

    /// The time to live of the RR.  This field [...] is primarily used by
    /// resolvers when they cache RRs.  The TTL describes how
    /// long a RR can be cached before it should be discarded.
    pub ttl: u32,

    /// The type and sometimes class dependent data
    /// which describes the resource
    /// Includes also the type of the resource in this resource record.
    /// Types refer to abstract resources.
    pub rdata: ResourceRecordData,
}

/// RFC 1035 - Section 3.3
#[derive(Debug, Clone)]
pub enum ResourceRecordData {
    /// RFC 1035 - Section 3.3.1
    ///
    /// A <domain-name> which specifies the canonical or primary
    /// name for the owner. The owner name is an alias.
    CNAME(DomainName),

    /// RFC 1035 - Section 3.3.2
    ///
    /// HINFO records are used to acquire general information about a host.
    /// The main use is for protocols such as FTP that can use special procedures
    /// when talking between machines or operating systems of the same type.
    HINFO {
        /// A <character-string> which specifies the CPU type.
        cpu: CharacterString,

        /// A <character-string> which specifies the operating system type.
        os: CharacterString,
    },

    /// RFC 1035 - Section 3.3.3 (EXPERIMENTAL)
    ///
    /// A <domain-name> which specifies a host which has the specified mailbox.
    MB(DomainName),

    /// RFC 1035 - Section 3.3.4 (Obsolete)
    ///
    /// A <domain-name> which specifies a host which has a mail
    /// agent for the domain which should be able to deliver
    /// mail for the domain.
    /// MD is obsolete.  See the definition of MX and RFC 974.
    MD(DomainName),

    /// RFC 1035 - Section 3.3.5 (Obsolete)
    ///
    /// A <domain-name> which specifies a host which has a mail
    /// agent for the domain which will accept mail for
    /// forwarding to the domain.
    /// MF is obsolete.  See the definition of MX and RFC 974.
    MF(DomainName),

    /// RFC 1035 - Section 3.3.6 (EXPERIMENTAL)
    ///
    /// A <domain-name> which specifies a mailbox which is a
    /// member of the mail group specified by the domain name.
    MG(DomainName),

    /// RFC 1035 - Section 3.3.7 (EXPERIMENTAL)
    ///
    /// Although these records can be associated with a simple mailbox,
    /// they are usually used with a mailing list.
    MINFO {
        ///  A <domain-name> which specifies a mailbox which is
        /// responsible for the mailing list or mailbox.  If this
        /// domain name names the root, the owner of the MINFO RR is
        /// responsible for itself.  Note that many existing mailing
        /// lists use a mailbox X-request for the RMAILBX field of
        /// mailing list X, e.g., Msgroup-request for Msgroup.  This
        /// field provides a more general mechanism.
        rmailbx: DomainName,

        /// A <domain-name> which specifies a mailbox which is to
        /// receive error messages related to the mailing list or
        /// mailbox specified by the owner of the MINFO RR (similar
        /// to the ERRORS-TO: field which has been proposed).  If
        /// this domain name names the root, errors should be
        /// returned to the sender of the message.
        emailbx: DomainName,
    },

    /// RFC 1035 - Section 3.3.8 (EXPERIMENTAL)
    ///
    /// A <domain-name> which specifies a mailbox which is the proper rename of the specified mailbox.
    /// The main use for MR is as a forwarding entry for a user who has moved to a different mailbox.
    MR(DomainName),

    /// RFC 1035 - Section 3.3.9
    ///
    /// The use of MX RRs is explained in detail in RFC 974.
    MX {
        /// A 16 bit integer which specifies the preference given to
        /// this RR among others at the same owner. Lower values are preferred.
        preference: u16,

        /// A <domain-name> which specifies a host willing to act as a mail exchange for the owner name.
        exchange: DomainName,
    },

    /// RFC 1035 - Section 3.3.10
    ///
    /// Anything at all may be in the RDATA field so long as it is 65535 octets or less.
    NULL(Vec<u8>),

    /// RFC 1035 - Section 3.3.11
    ///
    /// A <domain-name> which specifies a host which should be
    /// authoritative for the specified class and domain.
    /// The NS RR states that the named host should be expected to have a zone
    /// starting at owner name of the specified class.  Note that the class may
    /// not indicate the protocol family which should be used to communicate
    /// with the host, although it is typically a strong hint.  For example,
    /// hosts which are name servers for either Internet (IN) or Hesiod (HS)
    /// class information are normally queried using IN class protocols.
    NS(DomainName),

    /// RFC 1035 - Section 3.3.12
    ///
    /// A <domain-name> which points to some location in the domain name space.
    /// These RRs are used in special domains to point to some other location in the domain space.
    /// These records are simple data, and don't imply any special processing
    /// similar to that performed by CNAME, which identifies aliases.  See the
    /// description of the IN-ADDR.ARPA domain for an example.
    PTR(DomainName),

    /// RFC 1035 - Section 3.3.13
    SOA {
        /// The <domain-name> of the name server that was the original or primary source of data for this zone.
        mname: DomainName,

        /// A <domain-name> which specifies the mailbox of the person responsible for this zone.
        rname: DomainName,

        /// The unsigned 32 bit version number of the original copy
        /// of the zone.  Zone transfers preserve this value.  This
        /// value wraps and should be compared using sequence space
        /// arithmetic.
        serial: u32,

        /// A 32 bit time interval before the zone should be refreshed.
        refresh: u32,

        /// A 32 bit time interval that should elapse before a failed refresh should be retried.
        retry: u32,

        /// A 32 bit time value that specifies the upper limit on the time interval
        /// that can elapse before the zone is no longer authoritative.
        expire: u32,

        /// The unsigned 32 bit minimum TTL field that should be exported with any RR from this zone.
        minimum: u32,
    },

    /// RFC 1035 - Section 3.3.14
    ///
    /// One or more <character-string>s.
    /// TXT RRs are used to hold descriptive text.  The semantics of the text
    /// depends on the domain where it is found.
    TXT(Vec<CharacterString>),

    /// RFC 1035 - Section 3.4.1
    ///
    /// A 32 bit Internet address.
    /// Hosts that have multiple Internet addresses will have multiple A records.
    A(Ipv4Addr),

    /// RFC 1035 - Section 3.4.2
    ///
    /// The WKS record is used to describe the well known services supported by
    /// a particular protocol on a particular internet address.  The PROTOCOL
    /// field specifies an IP protocol number, and the bit map has one bit per
    /// port of the specified protocol.  The first bit corresponds to port 0,
    /// the second to port 1, etc.  If the bit map does not include a bit for a
    /// protocol of interest, that bit is assumed zero.  The appropriate values
    /// and mnemonics for ports and protocols are specified in RFC 1010.
    ///
    /// The purpose of WKS RRs is to provide availability information for
    /// servers for TCP and UDP.  If a server supports both TCP and UDP, or has
    /// multiple Internet addresses, then multiple WKS RRs are used.
    WKS {
        /// An 32 bit Internet address
        address: Ipv4Addr,

        /// An 8 bit IP protocol number
        protocol: u8,

        /// A variable length bit map.  The bit map must be a multiple of 8 bits long.
        bitmap: Vec<u8>,
    },

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
