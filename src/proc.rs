// convenience functions for getting information from /proc/net
use std::fs::File;
use std::net::Ipv4Addr;
use std::num::ParseIntError;
use std::fmt::{
    Display,
    Formatter,
    Result as FmtResult
};
use std::io::{
    Read,
    Error as IoError
};
use pnet::datalink::{
    MacAddr,
    ParseMacAddrErr
};

pub fn get_arp_entry(name: String) -> Result<MacAddr, ProcFileError> {
    let mut buf = String::new();

    File::open("/proc/net/arp")?.read_to_string(&mut buf)?;
        
    Ok(buf.split("\n")
          .filter(|_l| _l.get(0..16).unwrap_or("").trim() == name)
          .next()
          .ok_or(format!("{} entry in /proc/net/arp", name))?
          .get(41..58)
          .ok_or(String::from("HW address column in /proc/net/arp"))?
          .parse()?)  
}

pub fn get_gateway(name: &str) -> Result<Ipv4Addr, ProcFileError> {
    let mut buf = String::new();

    File::open("/proc/net/route")?.read_to_string(&mut buf)?;

    Ok(Ipv4Addr::from(u32::from_str_radix(
        buf.split("\n")
           .filter(|_l| _l.starts_with(name))
           .next()
           .ok_or(format!("{} entry in /proc/net/route", name))?
           .split("\t")
           .nth(2)
           .ok_or(String::from("gateway column in /proc/net/route"))?,
        16
    )?.to_be()))
}

pub enum ProcFileError {
    IO(IoError),
    Missing(String),
    Parse(ProcParseError)
}

pub enum ProcParseError {
    Mac(ParseMacAddrErr),
    Int(ParseIntError)
}

impl From<IoError> for  ProcFileError {
    fn from(e: IoError) -> Self {
        ProcFileError::IO(e)
    }
}

impl From<ParseIntError> for ProcFileError {
    fn from(e: ParseIntError) -> Self {
        ProcFileError::Parse(ProcParseError::Int(e))
    }
}

impl From<ParseMacAddrErr> for ProcFileError {
    fn from(e: ParseMacAddrErr) -> Self {
        ProcFileError::Parse(ProcParseError::Mac(e))
    }
}

impl From<String> for ProcFileError {
    fn from(s: String) -> Self {
        ProcFileError::Missing(s)
    }
}

impl Display for ProcFileError {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            ProcFileError::IO(e) => write!(f, "{}", e),
            ProcFileError::Missing(s) => write!(f, "missing {}", s),
            ProcFileError::Parse(parse) => match parse {
                ProcParseError::Int(e) => write!(f, "{}", e),
                ProcParseError::Mac(mac) => match mac {
                    ParseMacAddrErr::TooManyComponents => write!(f, "too many components in /proc/net/arp entry"),
                    ParseMacAddrErr::TooFewComponents => write!(f, "too few components in /proc/net/arp entry"),
                    ParseMacAddrErr::InvalidComponent => write!(f, "invalid component in /proc/net/arp entry")
                } 
            }
        }
    }
}