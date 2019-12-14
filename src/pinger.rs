use winapi::{
    shared::{
        minwindef::TRUE,
        ntdef::{HANDLE, NULL},
        ws2def::AF_INET6,
        ws2ipdef::SOCKADDR_IN6,
    },
    um::{
        handleapi::INVALID_HANDLE_VALUE,
        icmpapi::{
            Icmp6CreateFile, Icmp6SendEcho2, IcmpCloseHandle, IcmpCreateFile, IcmpSendEcho,
            IcmpSendEcho2Ex,
        },
        ipexport::{IP_FLAG_DF, IP_SUCCESS},
    },
};

#[cfg(target_pointer_width = "32")]
use winapi::um::ipexport::IP_OPTION_INFORMATION;
#[cfg(target_pointer_width = "64")]
use winapi::um::ipexport::IP_OPTION_INFORMATION32 as IP_OPTION_INFORMATION;

use std::{
    fmt::{self, Debug, Formatter},
    mem,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    sync::Arc,
};

use crate::{Buffer, Error};

struct Handles {
    v4: HANDLE,
    v6: HANDLE,
}
/// A pair of IP (v4 or v6) addresses, source and destination.
#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub enum IpPair {
    V4 { src: Ipv4Addr, dst: Ipv4Addr },
    V6 { src: Ipv6Addr, dst: Ipv6Addr },
}
/// A pinger that blocks when sending.
#[derive(Clone)]
pub struct Pinger {
    handles: Arc<Handles>,
    ttl: u8,
    df: bool,
    timeout: u32,
}
/// An error when creating a Pinger.
pub enum CreateError {
    /// The ICMPv4 handle could not be created.
    NoV4(Pinger),
    /// The ICMPv6 handle could not be created.
    NoV6(Pinger),
    /// Neither handle could be created.
    None,
}

impl Debug for CreateError {
    fn fmt(&self, out: &mut Formatter) -> fmt::Result {
        write!(
            out,
            "{}",
            match self {
                CreateError::None => "Failed to create ICMP V4 and V6 handles",
                CreateError::NoV4(_) => "Failed to create ICMP V4 handle",
                CreateError::NoV6(_) => "Failed to create ICMP V6 handle",
            }
        )
    }
}

impl Pinger {
    /// Creates a new Pinger.
    /// If one ICMP handle (v4 or v6) fails to initialize,
    /// this will return the Pinger embedded in an error. The
    /// Pinger is still usable in this state, but sending
    /// on the failed version will return an Error.
    /// If both v4 and v6 fail, the pinger is not embedded
    /// in the error.
    pub fn new() -> Result<Self, CreateError> {
        let (v4, v6) = unsafe { (IcmpCreateFile(), Icmp6CreateFile()) };
        let ret = Self {
            handles: Arc::new(Handles { v4, v6 }),
            ttl: 255,
            df: false,
            timeout: 2000,
        };
        match (v4, v6) {
            (INVALID_HANDLE_VALUE, INVALID_HANDLE_VALUE) => Err(CreateError::None),
            (INVALID_HANDLE_VALUE, _) => Err(CreateError::NoV6(ret)),
            (_, INVALID_HANDLE_VALUE) => Err(CreateError::NoV4(ret)),
            (_, _) => Ok(ret),
        }
    }
    /// Creates a new Pinger, ignoring v6 failures. If you want to use
    /// both v4 and v6, use new() instead.
    pub fn new_v4() -> Option<Self> {
        match Self::new() {
            Ok(ret) | Err(CreateError::NoV6(ret)) => Some(ret),
            _ => None,
        }
    }
    /// Creates a new Pinger, ignoring v4 failures. If you want to use
    /// both v4 and v6, use new() instead.
    pub fn new_v6() -> Option<Self> {
        match Self::new() {
            Ok(ret) | Err(CreateError::NoV4(ret)) => Some(ret),
            _ => None,
        }
    }
    /// Sets the IP TTL for future requets.
    pub fn set_ttl(&mut self, ttl: u8) {
        self.ttl = ttl;
    }
    /// Gets the current IP TTL value.
    pub fn ttl(&self) -> u8 {
        self.ttl
    }
    /// Sets the IP Don't Fragment bit for future requests.
    pub fn set_df(&mut self, df: bool) {
        self.df = df;
    }
    /// Gets the current IP Don't Fragment bit.
    pub fn df(&self) -> bool {
        self.df
    }
    /// Sets the timeout, in milliseconds, for future requests.
    pub fn set_timeout(&mut self, timeout: u32) {
        self.timeout = timeout;
    }
    /// Gets the current timeout in milliseconds.
    pub fn timeout(&self) -> u32 {
        self.timeout
    }
    #[inline]
    fn make_ip_opts(&self) -> IP_OPTION_INFORMATION {
        IP_OPTION_INFORMATION {
            Ttl: self.ttl,
            Flags: if self.df { IP_FLAG_DF } else { 0 },
            ..Default::default()
        }
    }
    /// Send an ICMPv4 request to the destination address.
    pub fn send4(&self, dst: Ipv4Addr, buf: &mut Buffer) -> Result<u32, Error> {
        buf.init_for_send();
        let ret = unsafe {
            IcmpSendEcho(
                self.handles.v4,
                mem::transmute(dst),
                buf.request_data_ptr(),
                buf.request_data_len(),
                &mut self.make_ip_opts(),
                buf.reply_data_ptr(),
                buf.reply_data_len(),
                self.timeout,
            )
        };
        if ret == 0 {
            Err(Error::from_lasterror())
        } else {
            let reply = buf.as_echo_reply().unwrap();
            if reply.Status == IP_SUCCESS {
                Ok(reply.RoundTripTime)
            } else {
                Err(Error::from_iperror(reply.Status))
            }
        }
    }
    /// Sends an ICMPv4 request from the source address to the destination address.
    pub fn send4_from(&self, src: Ipv4Addr, dst: Ipv4Addr, buf: &mut Buffer) -> Result<u32, Error> {
        buf.init_for_send();
        let ret = unsafe {
            IcmpSendEcho2Ex(
                self.handles.v4,
                NULL,      // Event
                NULL as _, // ApcRoutine
                NULL,      // ApcContext
                mem::transmute(src),
                mem::transmute(dst),
                buf.request_data_ptr(),
                buf.request_data_len(),
                &mut self.make_ip_opts(),
                buf.reply_data_ptr(),
                buf.reply_data_len(),
                self.timeout,
            )
        };
        if ret == 0 {
            Err(Error::from_lasterror())
        } else {
            let reply = buf.as_echo_reply().unwrap();
            if reply.Status == IP_SUCCESS {
                Ok(reply.RoundTripTime)
            } else {
                Err(Error::from_iperror(reply.Status))
            }
        }
    }
    /// Sends an ICMPv6 request to the destination address.
    pub fn send6(&self, dst: Ipv6Addr, buf: &mut Buffer) -> Result<u32, Error> {
        let mut dst = SOCKADDR_IN6 {
            sin6_family: AF_INET6 as _,
            sin6_addr: unsafe { mem::transmute(dst) },
            ..Default::default()
        };
        buf.init_for_send();

        let ret = unsafe {
            Icmp6SendEcho2(
                self.handles.v6,
                NULL,      // Event
                NULL as _, // ApcRoutine
                NULL,      // ApcContext
                &mut SOCKADDR_IN6::default(),
                &mut dst,
                buf.request_data_ptr(),
                buf.request_data_len(),
                &mut self.make_ip_opts(),
                buf.reply_data_ptr(),
                buf.reply_data_len(),
                self.timeout,
            )
        };
        if ret == 0 {
            Err(Error::from_lasterror())
        } else {
            let reply = buf.as_echo_reply6().unwrap();
            if reply.Status == IP_SUCCESS {
                Ok(reply.RoundTripTime as u32)
            } else {
                Err(Error::from_iperror(reply.Status))
            }
        }
    }
    /// Sends an ICMPv6 request from the source address to the destination address.
    pub fn send6_from(&self, src: Ipv6Addr, dst: Ipv6Addr, buf: &mut Buffer) -> Result<u32, Error> {
        let mut dst = SOCKADDR_IN6 {
            sin6_family: AF_INET6 as _,
            sin6_addr: unsafe { mem::transmute(dst) },
            ..Default::default()
        };
        let mut src = SOCKADDR_IN6 {
            sin6_family: AF_INET6 as _,
            sin6_addr: unsafe { mem::transmute(src) },
            ..Default::default()
        };
        buf.init_for_send();

        let ret = unsafe {
            Icmp6SendEcho2(
                self.handles.v6,
                NULL,      // Event
                NULL as _, // ApcRoutine
                NULL,      // ApcContext
                &mut src,
                &mut dst,
                buf.request_data_ptr(),
                buf.request_data_len(),
                &mut self.make_ip_opts(),
                buf.reply_data_ptr(),
                buf.reply_data_len(),
                self.timeout,
            )
        };
        if ret == 0 {
            Err(Error::from_lasterror())
        } else {
            let reply = buf.as_echo_reply6().unwrap();
            if reply.Status == IP_SUCCESS {
                Ok(reply.RoundTripTime as u32)
            } else {
                Err(Error::from_iperror(reply.Status))
            }
        }
    }
    /// Sends an ICMP request to the destination address. Supports both v4 and v6.
    pub fn send(&self, dst: IpAddr, buf: &mut Buffer) -> Result<u32, Error> {
        match dst {
            IpAddr::V4(ip) => self.send4(ip, buf),
            IpAddr::V6(ip) => self.send6(ip, buf),
        }
    }
    /// Sends an ICMP request from the source address to the destination address. Supports both v4 and v6.
    pub fn send_from(&mut self, src_dst_pair: IpPair, buf: &mut Buffer) -> Result<u32, Error> {
        match src_dst_pair {
            IpPair::V4 { src, dst } => self.send4_from(src, dst, buf),
            IpPair::V6 { src, dst } => self.send6_from(src, dst, buf),
        }
    }
}

impl Drop for Handles {
    fn drop(&mut self) {
        if self.v4 != INVALID_HANDLE_VALUE {
            let ret = unsafe { IcmpCloseHandle(self.v4) };
            debug_assert_eq!(TRUE, ret);
        }
        if self.v6 != INVALID_HANDLE_VALUE {
            let ret = unsafe { IcmpCloseHandle(self.v6) };
            debug_assert_eq!(TRUE, ret);
        }
    }
}
