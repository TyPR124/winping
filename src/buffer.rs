use static_assertions::{assert_impl_all, const_assert, const_assert_eq};
#[cfg(target_pointer_width = "64")]
use winapi::um::ipexport::ICMP_ECHO_REPLY32;
use winapi::{
    shared::ntdef::VOID,
    um::ipexport::{ICMPV6_ECHO_REPLY, ICMP_ECHO_REPLY},
};

use std::{
    mem::{self, align_of, size_of},
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
};

// Chunk is a lump of u8, apropriately sized and aligned
// for the necessary ICMP(V6)_ECHO_REPLY(32) types on
// 32 and 64 bit platforms.
#[cfg(target_pointer_width = "64")]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(C, align(8))]
struct Chunk([u8; 8]);
#[cfg(target_pointer_width = "32")]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(C, align(4))]
struct Chunk([u8; 4]);
// Prove it
const CHUNK_SIZE: usize = size_of::<Chunk>();
const_assert_eq!(CHUNK_SIZE, align_of::<Chunk>());
const_assert_eq!(CHUNK_SIZE, align_of::<ICMP_ECHO_REPLY>());
const_assert!(CHUNK_SIZE >= align_of::<ICMPV6_ECHO_REPLY>());
#[cfg(target_pointer_width = "64")]
const_assert!(CHUNK_SIZE >= align_of::<ICMP_ECHO_REPLY32>());
/// A buffer for request and reply data.
#[derive(Clone, Debug)]
pub struct Buffer {
    pub request_data: Vec<u8>,
    reply_data: Vec<Chunk>,
    state: ReplyState,
}
assert_impl_all!(Buffer: Send, Sync);
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum ReplyState {
    Empty,
    Filled4(usize),
    Filled6(usize),
}

impl Default for Buffer {
    fn default() -> Self {
        Self::new()
    }
}

impl Buffer {
    pub const fn new() -> Self {
        Self {
            request_data: Vec::new(),
            reply_data: Vec::new(),
            state: ReplyState::Empty,
        }
    }
    pub const fn with_data(data: Vec<u8>) -> Self {
        Self {
            request_data: data,
            reply_data: Vec::new(),
            state: ReplyState::Empty,
        }
    }

    pub(crate) fn init_for_send(&mut self) {
        // Reply buffer must be large enough for:
        // 1. Any possible ICMP_ECHO(V6)_REPLY(32) type +
        // 2. An ICMP error (8 bytes) +
        // 3. An IO_STATUS_BLOCK (up to 16 bytes) +
        // 4. The length of the request data
        #[cfg(target_pointer_width = "64")]
        const MIN_ECHO_REPLY_SIZE: usize = {
            const_assert!(size_of::<ICMPV6_ECHO_REPLY>() <= size_of::<ICMP_ECHO_REPLY>());
            const_assert!(size_of::<ICMP_ECHO_REPLY32>() <= size_of::<ICMP_ECHO_REPLY>());
            size_of::<ICMP_ECHO_REPLY>()
        };
        #[cfg(target_pointer_width = "32")]
        const MIN_ECHO_REPLY_SIZE: usize = {
            const_assert!(size_of::<ICMP_ECHO_REPLY>() <= size_of::<ICMPV6_ECHO_REPLY>());
            size_of::<ICMPV6_ECHO_REPLY>()
        };
        const BASE_SIZE: usize = MIN_ECHO_REPLY_SIZE + 24;
        let size = BASE_SIZE + self.request_data.len();

        let chunks = size / CHUNK_SIZE
            + match size % CHUNK_SIZE {
                0 => 0,
                _ => 1,
            };
        self.reply_data.resize(chunks, Chunk([0; CHUNK_SIZE]));
        self.state = ReplyState::Empty;
    }
    pub(crate) fn request_data_ptr(&self) -> *mut VOID {
        let ptr: *const u8 = self.request_data.as_ptr();
        ptr as *mut VOID
    }
    pub(crate) fn request_data_len(&self) -> u16 {
        self.request_data.len() as u16
    }
    pub(crate) fn reply_data_ptr(&self) -> *mut VOID {
        let ptr: *const Chunk = self.reply_data.as_ptr();
        ptr as *mut VOID
    }
    pub(crate) fn reply_data_len(&self) -> u32 {
        (self.reply_data.len() * CHUNK_SIZE) as u32
    }
    pub(crate) fn as_echo_reply(&self) -> Option<&ICMP_ECHO_REPLY> {
        if self.reply_data_len() as usize >= size_of::<ICMP_ECHO_REPLY>() {
            Some(unsafe { &*self.reply_data_ptr().cast() })
        } else {
            None
        }
    }
    #[cfg(all(target_pointer_width = "64", not(feature = "no_async")))]
    pub(crate) fn as_echo_reply32(&self) -> Option<&ICMP_ECHO_REPLY32> {
        if self.reply_data_len() as usize >= size_of::<ICMP_ECHO_REPLY32>() {
            Some(unsafe { &*self.reply_data_ptr().cast() })
        } else {
            None
        }
    }
    pub(crate) fn as_echo_reply6(&self) -> Option<&ICMPV6_ECHO_REPLY> {
        if self.reply_data_len() as usize >= size_of::<ICMPV6_ECHO_REPLY>() {
            Some(unsafe { &*self.reply_data_ptr().cast() })
        } else {
            None
        }
    }
    pub(crate) fn set_filled4(&mut self) {
        self.state = ReplyState::Filled4(self.request_data.len())
    } 
    pub(crate) fn set_filled6(&mut self) {
        self.state = ReplyState::Filled6(self.request_data.len())
    }
    pub fn reply_data(&self) -> &[u8] {
        let (len, offset) = match self.state {
            ReplyState::Empty => (0, 0),
            ReplyState::Filled4(len) => {
                (len, size_of::<ICMP_ECHO_REPLY>())
            }
            ReplyState::Filled6(len) => {
                // RFC 4443, section 4.2, reply data MUST be same as request data
                (len, size_of::<ICMPV6_ECHO_REPLY>())
            }
        };

        if len == 0 || offset + len > self.reply_data_len() as usize {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(self.reply_data_ptr().cast::<u8>().add(offset), len) }
        }
    }
    pub fn responding_ipv4(&self) -> Option<Ipv4Addr> {
        match self.state {
            ReplyState::Filled4(..) => {
                let ipv4: Ipv4Addr = unsafe {mem::transmute(self.as_echo_reply().unwrap().Address)};
                Some(ipv4)
            }
            _ => None
        }
    }
    pub fn responding_ipv6(&self) -> Option<Ipv6Addr> {
        match self.state {
            ReplyState::Filled6(..) => {
                let ipv6: Ipv6Addr = unsafe {mem::transmute(self.as_echo_reply6().unwrap().Address.sin6_addr)};
                Some(ipv6)
            }
            _ => None
        }
    }
    pub fn responding_ip(&self) -> Option<IpAddr> {
        self.responding_ipv4().map(IpAddr::V4).or(self.responding_ipv6().map(IpAddr::V6))
    }
}
