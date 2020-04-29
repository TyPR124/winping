use winapi::{
    shared::{
        ntdef::NULL,
        winerror::{
            ERROR_HOST_UNREACHABLE, ERROR_NETWORK_UNREACHABLE, ERROR_PROTOCOL_UNREACHABLE, NO_ERROR,
        },
    },
    um::{
        errhandlingapi::GetLastError,
        ipexport::{
            IP_DEST_HOST_UNREACHABLE, IP_DEST_NET_UNREACHABLE, IP_DEST_PROT_UNREACHABLE,
            IP_PACKET_TOO_BIG, IP_REQ_TIMED_OUT, IP_STATUS_BASE, IP_TTL_EXPIRED_REASSEM,
            IP_TTL_EXPIRED_TRANSIT, MAX_IP_STATUS,
        },
        iphlpapi::GetIpErrorString,
        winbase::{FormatMessageW, FORMAT_MESSAGE_FROM_SYSTEM, FORMAT_MESSAGE_IGNORE_INSERTS},
    },
};

use std::fmt::{self, Debug, Display, Formatter};

/// An error when sending a ping request.
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub enum Error {
    /// The ping request timed out.
    Timeout,
    /// The destination network is unreachable.
    NetUnreachable,
    /// The destination host is unreachable.
    HostUnreachable,
    /// The IP TTL expired during transit.
    TtlExpired,
    /// The IP reassembly timer expired.
    ReassemblyExpired,
    /// The packet needs fragmented, but the DF bit is set.
    NeedsFragmented,
    /// The destination protocol is unreachable.
    ProtocolUnreachable,
    /// Some other error ocurred. Format with debug or diplay to get more info.
    Other(u32),
}

impl Error {
    pub(crate) fn from_iperror(err: u32) -> Self {
        match err {
            IP_REQ_TIMED_OUT => Error::Timeout,
            IP_DEST_HOST_UNREACHABLE => Error::HostUnreachable,
            IP_DEST_NET_UNREACHABLE => Error::NetUnreachable,
            IP_TTL_EXPIRED_TRANSIT => Error::TtlExpired,
            IP_TTL_EXPIRED_REASSEM => Error::ReassemblyExpired,
            IP_DEST_PROT_UNREACHABLE => Error::ProtocolUnreachable,
            IP_PACKET_TOO_BIG => Error::NeedsFragmented,
            _ => Error::Other(err),
        }
    }
    /// Creates an Error from the last Windows error
    pub(crate) fn from_lasterror() -> Self {
        Self::from_winerror(unsafe { GetLastError() })
    }
    /// Takes either a Windows error or IP_STATUS value
    pub(crate) fn from_winerror(err: u32) -> Self {
        match err {
            IP_STATUS_BASE..=MAX_IP_STATUS => Error::from_iperror(err),
            ERROR_HOST_UNREACHABLE => Error::HostUnreachable,
            ERROR_NETWORK_UNREACHABLE => Error::NetUnreachable,
            ERROR_PROTOCOL_UNREACHABLE => Error::ProtocolUnreachable,
            _ => Error::Other(err),
        }
    }
}

impl Debug for Error {
    fn fmt(&self, out: &mut Formatter) -> fmt::Result {
        match self {
            Error::Timeout => write!(out, "Request timed out"),
            Error::HostUnreachable => write!(out, "Destination host unreachable"),
            Error::NetUnreachable => write!(out, "Destination network unreachable"),
            Error::TtlExpired => write!(out, "TTL expired in transit"),
            Error::ReassemblyExpired => write!(out, "Reassembly timed out waiting for fragments"),
            Error::NeedsFragmented => write!(out, "Packet needs fragmented"),
            Error::ProtocolUnreachable => write!(out, "Destination protocol unreachable"),
            Error::Other(err @ IP_STATUS_BASE..=MAX_IP_STATUS) => {
                let mut buf = [0u16; 256];
                let ret =
                    unsafe { GetIpErrorString(*err, &mut buf[0], &mut (buf.len() as u32 - 1)) };
                debug_assert_eq!(NO_ERROR, ret);
                let len = buf.iter().take_while(|x| **x != 0).count();
                let s = String::from_utf16_lossy(&buf[..len]);
                write!(out, "Other IP error ({}): {}", err, s.trim())
            }
            Error::Other(err) => {
                const FLAGS: u32 = FORMAT_MESSAGE_FROM_SYSTEM | FORMAT_MESSAGE_IGNORE_INSERTS;
                let mut buf = [0u16; 256];
                let len = unsafe {
                    FormatMessageW(
                        FLAGS,
                        NULL,
                        *err,
                        0, // lang id
                        &mut buf[0],
                        buf.len() as u32,
                        NULL as _,
                    )
                };
                let s = String::from_utf16_lossy(&buf[..len as usize]);
                write!(out, "Other error ({}): {}", err, s.trim())
            }
        }
    }
}

impl Display for Error {
    fn fmt(&self, out: &mut Formatter) -> fmt::Result {
        Debug::fmt(self, out)
    }
}

impl std::error::Error for Error {}
