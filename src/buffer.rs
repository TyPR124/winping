use static_assertions::{assert_impl_all, const_assert, const_assert_eq};
#[cfg(target_pointer_width = "64")]
use winapi::um::ipexport::ICMP_ECHO_REPLY32;
use winapi::{
    shared::ntdef::VOID,
    um::ipexport::{ICMPV6_ECHO_REPLY, ICMP_ECHO_REPLY},
};

use std::mem::{align_of, size_of};

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
    pub(crate) fn set_has_echo_reply(&mut self) -> Option<&ICMP_ECHO_REPLY> {
        if self.reply_data_len() as usize >= size_of::<ICMP_ECHO_REPLY>() {
            self.state = ReplyState::Filled4(self.request_data.len());
            Some(unsafe { &*self.reply_data_ptr().cast() })
        } else {
            None
        }
    }
    #[cfg(all(target_pointer_width = "64", not(feature = "no_async")))]
    pub(crate) fn set_has_echo_reply32(&mut self) -> Option<&ICMP_ECHO_REPLY32> {
        if self.reply_data_len() as usize >= size_of::<ICMP_ECHO_REPLY32>() {
            // ReplyState does not need to differentiate echo_reply32.
            // The reply data is stored after an ICMP_ECHO_REPLY (not 32), and
            // is not moved when IcmpParseReplies converts it to ICMP_ECHO_REPLY32
            self.state = ReplyState::Filled4(self.request_data.len());
            Some(unsafe { &*self.reply_data_ptr().cast() })
        } else {
            None
        }
    }
    pub(crate) fn set_has_echo_reply6(&mut self) -> Option<&ICMPV6_ECHO_REPLY> {
        if self.reply_data_len() as usize >= size_of::<ICMPV6_ECHO_REPLY>() {
            self.state = ReplyState::Filled6(self.request_data.len());
            Some(unsafe { &*self.reply_data_ptr().cast() })
        } else {
            None
        }
    }
    pub fn reply_data(&self) -> &[u8] {
        let (len, offset) = match self.state {
            ReplyState::Empty => (0, 0),
            ReplyState::Filled4(len) => {
                (len, size_of::<ICMP_ECHO_REPLY>())
            }
            ReplyState::Filled6(len) => {
                (len, size_of::<ICMPV6_ECHO_REPLY>())
            }
        };

        if len == 0 || offset + len > self.reply_data_len() as usize {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(self.reply_data_ptr().cast::<u8>().add(offset), len) }
        }
    }
}
