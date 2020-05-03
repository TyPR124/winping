use winapi::{
    shared::{
        minwindef::{DWORD, TRUE},
        ntdef::{HANDLE, NULL, ULONG, VOID},
        winerror::ERROR_IO_PENDING,
        ws2def::AF_INET6,
        ws2ipdef::SOCKADDR_IN6,
    },
    um::{
        errhandlingapi::GetLastError,
        handleapi::INVALID_HANDLE_VALUE,
        icmpapi::{
            Icmp6CreateFile, Icmp6ParseReplies, Icmp6SendEcho2, IcmpCreateFile, IcmpParseReplies,
            IcmpSendEcho2, IcmpSendEcho2Ex,
        },
        ipexport::{IP_FLAG_DF, IP_SUCCESS},
        synchapi::{CreateEventExW, SetEvent, WaitForSingleObjectEx},
        winbase::{INFINITE, WAIT_FAILED, WAIT_IO_COMPLETION, WAIT_OBJECT_0},
        winnt::{DELETE, EVENT_MODIFY_STATE, SYNCHRONIZE},
    },
};

#[cfg(target_pointer_width = "32")]
use winapi::um::ipexport::IP_OPTION_INFORMATION;
#[cfg(target_pointer_width = "64")]
use winapi::um::ipexport::IP_OPTION_INFORMATION32 as IP_OPTION_INFORMATION;

use lazy_static::lazy_static;
use static_assertions::assert_impl_all;

use std::{
    future::Future,
    marker::Unpin,
    mem::{self, replace},
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    pin::Pin,
    sync::mpsc::{self, Receiver, SyncSender},
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
    thread,
};

/* For future reference:
 *
 * The approach taken (see doc on AsyncPinger::new) and the reason I don't use
 * simple event-based async is for several reasons:
 *  1. Rust async specifies futures can be polled once and only checked again
 *     after being awaken. For this reason, using an event doesn't work as there
 *     is no callback when waiting for an event.
 *  2. Using a callback-based approach in the calling thread (as apposed to spawning
 *     a thread) only works if the executor uses wait functions in a certain way,
 *     which I do not think can be relied on.
 *  3. Using event's in the new thread (as apposed to callbacks) would limit the number
 *     of events that could waited on.
 *
 * Therefore, my approach is to spawn a new thread, and and handle async via callback
 * for all async pingers in that thread.
 *
 * State (which may be mutated by either calling thread or spawned thread) is stored in
 * an Arc<Mutex> to ensure (1) lack of data races, and (2) either thread dropping the
 * state does not invalidate the other.
 *
 */

use crate::{Buffer, Error, IpPair};
/// A pinger that does not block when sending.
#[derive(Clone)]
pub struct AsyncPinger {
    worker: Worker,
    ttl: u8,
    df: bool,
    timeout: u32,
}
/// The result of an async ping. Contains a Result, and the
/// buffer that was originally passed into the pinger.
#[must_use]
pub struct AsyncResult {
    #[must_use]
    pub result: Result<u32, Error>,
    pub buffer: Buffer,
}
/// The immediate return value of an AsyncPinger. You should probably just use
/// async/await syntax instead.
pub struct PingFuture {
    state: Arc<Mutex<State>>,
    kind: IpKind,
}
assert_impl_all!(PingFuture: Send, Unpin);

enum State {
    Unpolled(Buffer),
    Polled(Buffer, Waker),
    Ready(Buffer),
    Failed(Buffer, u32),
    FailedAsyncSend(Buffer, u32),
    Invalid,
}
// Expected State Transitions
// Initial state: Unpolled
// Unpolled -> FailedAsyncSend if IcmpSend* returns unexpected value
// Unpolled -> Failed if IcmpSend* returns error (other than IO_PENDING)
// Unpolled -> Ready if not yet polled and callback_fn completes
// Unpolled -> Polled if not yet polled when polled
// Polled -> Polled if already polled when polled
// Polled -> Ready if already polled and callback_fn completes

impl AsyncPinger {
    /// Creates a new AsyncPinger.
    /// Creating one or more AsyncPingers will spawn
    /// a single dedicated thread which handles all async IO for all AsyncPingers.
    /// If ICMP handle initialization fails, all ping requests will return
    /// an error.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            worker: Worker::new(),
            ttl: 255,
            df: false,
            timeout: 2000,
        }
    }
    /// Sets the IP TTL for future requets.
    pub fn set_ttl(&mut self, ttl: u8) {
        self.ttl = ttl
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
    /// Send an ICMPv4 request to the destination address. On success, returns the round trip time in milliseconds.
    pub fn send4(&self, dst: Ipv4Addr, mut buf: Buffer) -> PingFuture {
        buf.init_for_send();
        self.worker
            .begin_v4(None, dst, buf, self.ttl, self.timeout, self.df)
    }
    /// Sends an ICMPv4 request from the source address to the destination address. On success, returns the round trip time in milliseconds.
    pub fn send4_from(&self, src: Ipv4Addr, dst: Ipv4Addr, mut buf: Buffer) -> PingFuture {
        buf.init_for_send();
        self.worker
            .begin_v4(Some(src), dst, buf, self.ttl, self.timeout, self.df)
    }
    /// Sends an ICMPv6 request to the destination address. On success, returns the round trip time in milliseconds.
    pub fn send6(&self, dst: Ipv6Addr, mut buf: Buffer) -> PingFuture {
        buf.init_for_send();
        self.worker
            .begin_v6(None, dst, buf, self.ttl, self.timeout, self.df)
    }
    /// Sends an ICMPv6 request from the source address to the destination address. On success, returns the round trip time in milliseconds.
    pub fn send6_from(&self, src: Ipv6Addr, dst: Ipv6Addr, mut buf: Buffer) -> PingFuture {
        buf.init_for_send();
        self.worker
            .begin_v6(Some(src), dst, buf, self.ttl, self.timeout, self.df)
    }
    /// Sends an ICMP request to the destination address. Supports both v4 and v6. On success, returns the round trip time in milliseconds.
    pub fn send(&self, dst: IpAddr, buf: Buffer) -> PingFuture {
        match dst {
            IpAddr::V4(dst) => self.send4(dst, buf),
            IpAddr::V6(dst) => self.send6(dst, buf),
        }
    }
    /// Sends an ICMP request from the source address to the destination address. Supports both v4 and v6. On success, returns the round trip time in milliseconds.
    pub fn send_from(&self, src_dst_pair: IpPair, buf: Buffer) -> PingFuture {
        match src_dst_pair {
            IpPair::V4 { src, dst } => self.send4_from(src, dst, buf),
            IpPair::V6 { src, dst } => self.send6_from(src, dst, buf),
        }
    }
}

#[derive(Copy, Clone)]
enum IpOptionalPair {
    V4 {
        src: Option<Ipv4Addr>,
        dst: Ipv4Addr,
    },
    V6 {
        src: Option<Ipv6Addr>,
        dst: Ipv6Addr,
    },
}
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum IpKind {
    V4,
    V6,
}
#[derive(Clone)]
struct Worker {
    inner: SyncSender<Job>,
}
struct Job {
    pair: IpOptionalPair,
    data_ptr: *mut VOID,
    data_len: u16,
    reply_ptr: *mut VOID,
    reply_len: u32,
    ttl: u8,
    timeout: u32,
    df: bool,
    cx: Arc<Mutex<State>>,
}
unsafe impl Send for Job {}

impl Worker {
    fn new() -> Self {
        let inner = ASYNC_SENDER.lock().unwrap().clone();
        Self { inner }
    }
    fn begin_v4(
        &self,
        src: Option<Ipv4Addr>,
        dst: Ipv4Addr,
        mut buf: Buffer,
        ttl: u8,
        timeout: u32,
        df: bool,
    ) -> PingFuture {
        let data_ptr = buf.request_data_ptr();
        let data_len = buf.request_data_len();
        let reply_ptr = buf.reply_data_ptr();
        let reply_len = buf.reply_data_len();
        let state = Arc::new(Mutex::new(State::Unpolled(buf)));
        let cx = state.clone();
        let job = Job {
            pair: IpOptionalPair::V4 { src, dst },
            data_ptr,
            data_len,
            reply_ptr,
            reply_len,
            ttl,
            timeout,
            df,
            cx,
        };
        self.inner.send(job).unwrap();
        unsafe { SetEvent(INPUT_EVENT) };
        PingFuture {
            state,
            kind: IpKind::V4,
        }
    }
    fn begin_v6(
        &self,
        src: Option<Ipv6Addr>,
        dst: Ipv6Addr,
        mut buf: Buffer,
        ttl: u8,
        timeout: u32,
        df: bool,
    ) -> PingFuture {
        let data_ptr = buf.request_data_ptr();
        let data_len = buf.request_data_len();
        let reply_ptr = buf.reply_data_ptr();
        let reply_len = buf.reply_data_len();
        let state = Arc::new(Mutex::new(State::Unpolled(buf)));
        let cx = state.clone();
        let job = Job {
            pair: IpOptionalPair::V6 { src, dst },
            data_ptr,
            data_len,
            reply_ptr,
            reply_len,
            ttl,
            timeout,
            df,
            cx,
        };
        self.inner.send(job).unwrap();
        unsafe { SetEvent(INPUT_EVENT) };
        PingFuture {
            state,
            kind: IpKind::V6,
        }
    }
}

static mut INPUT_EVENT: HANDLE = NULL;
static mut ICMP_HANDLE: HANDLE = INVALID_HANDLE_VALUE;
static mut ICMP6_HANDLE: HANDLE = INVALID_HANDLE_VALUE;

// The size of the async channel buffer is determined by one of three possible methods.
// The lowest priority method is a static default value.
// The next method is an optional compile-time defined value.
// The highest priority method is an optional run-time value.

static ASYNC_BUFFER_SIZE_CT: Option<&'static str> = std::option_env!("WINPING_ASYNC_BUFFER_SIZE");
/// This variable can be modified at run-time
/// to determine the size of the inter-thread buffer to use
/// for AsyncPinger. This buffer is specifically used for sending
/// jobs (ping requests) to the thread which handles the async IO
/// (as described in docs for `AsyncPinger::new`). As such, this
/// is a a variable which, if used, must be set prior to creating
/// an AsyncPinger, and once set it will override any compile-time
/// value (which can be set by defining a compile-time environment
/// variable named WINPING_ASYNC_BUFFER_SIZE). If neither the
/// compile-time nor the run-time values are set, AsyncPinger falls
/// back on a default value of 1024.
///
/// Note that if the compile-time environment variable is set and
/// cannot be parsed, this will result in a run-time panic the first
/// time an AsyncPinger is created!
///
/// # Safety
///
/// It is unsafe to set this variable because it is global and mutable
/// with no protection against data races. If you set this variable,
/// it MUST be done prior to creating any AsyncPinger.
pub static mut ASYNC_BUFFER_SIZE: Option<usize> = None;
static ASYNC_BUFFER_SIZE_DEFAULT: usize = 1024;

lazy_static! {
    static ref ASYNC_SENDER: Mutex<SyncSender<Job>> = {
        // Safety: reading value of pub static mut ASYNC_BUFFER_SIZE - it is up to user to not cause data-races, as described
        // in docs for the variable.
        let channel_size = unsafe { ASYNC_BUFFER_SIZE.unwrap_or_else(||
            ASYNC_BUFFER_SIZE_CT.map_or(ASYNC_BUFFER_SIZE_DEFAULT, |s| s.parse().expect("Failed to parse value of WINPING_ASYNC_BUFFER_SIZE compile-time environment variable"))
        )};
        let (tx, rx) = mpsc::sync_channel(channel_size);
        const EVENT_ACCESS: DWORD = DELETE | EVENT_MODIFY_STATE | SYNCHRONIZE;
        unsafe {
            INPUT_EVENT = CreateEventExW(NULL as _, NULL as _, 0, EVENT_ACCESS);
            if INPUT_EVENT == NULL { panic!("Could not initialize event handle") }
            // Ignore failures for ICMP handles - instead, allow IcmpSendEcho (and similar) to error on use
            ICMP_HANDLE = IcmpCreateFile();
            ICMP6_HANDLE = Icmp6CreateFile();
        }
        let ret = Mutex::new(tx);

        thread::spawn(move||loop {
            // WaitForSingleObjectEx returns if INPUT_EVENT is signaled, or if callback_fn is called
            match unsafe { WaitForSingleObjectEx(INPUT_EVENT, INFINITE, TRUE) } {
                WAIT_IO_COMPLETION | WAIT_OBJECT_0 => while try_recv_job(&rx) {},
                WAIT_FAILED => {
                    let err = Error::from_lasterror();
                    panic!("AsyncPinger thread failed to wait in event loop: {}", err)
                }
                x => unreachable!("unexpected return from WaitForSingleObjectEx: {:x}", x)
            }
        });

        ret
    };
}

#[inline]
fn try_recv_job(rx: &Receiver<Job>) -> bool {
    let job = match rx.try_recv() {
        Ok(job) => job,
        _ => return false,
    };
    let mut ip_opts = IP_OPTION_INFORMATION {
        Ttl: job.ttl,
        Flags: if job.df { IP_FLAG_DF } else { 0 },
        ..Default::default()
    };
    let arcptr = Arc::into_raw(job.cx);

    #[inline]
    fn after_send(ret: u32, arcptr: *const Mutex<State>) {
        if ret != 0 {
            let arc = unsafe { Arc::from_raw(arcptr) };
            let mut lock = arc.lock().unwrap();
            let state = replace(&mut *lock, State::Invalid);
            match state {
                State::Unpolled(buf) => *lock = State::FailedAsyncSend(buf, ret),
                State::Polled(buf, waker) => {
                    *lock = State::FailedAsyncSend(buf, ret);
                    waker.wake();
                }
                _ => {} // Leave state as Invalid, pushes panic out of async thread
            }
        }
        let err = unsafe { GetLastError() };
        if err != ERROR_IO_PENDING {
            let arc = unsafe { Arc::from_raw(arcptr) };
            let mut lock = arc.lock().unwrap();
            let state = replace(&mut *lock, State::Invalid);
            match state {
                State::Unpolled(buf) => *lock = State::Failed(buf, err),
                State::Polled(buf, waker) => {
                    *lock = State::Failed(buf, err);
                    waker.wake();
                }
                _ => {} // Leave state as Invalid, pushes panic out of async thread
            }
        }
    }

    use IpOptionalPair::{V4, V6};
    match job.pair {
        V4 {
            src: Some(src),
            dst,
        } => {
            let ret = unsafe {
                IcmpSendEcho2Ex(
                    ICMP_HANDLE,
                    NULL,             // Event
                    callback_fn as _, // ApcRoutine,
                    arcptr as _,      // ApcContext,
                    mem::transmute(src),
                    mem::transmute(dst),
                    job.data_ptr,
                    job.data_len,
                    &mut ip_opts,
                    job.reply_ptr,
                    job.reply_len,
                    job.timeout,
                )
            };
            after_send(ret, arcptr);
        }
        V4 { src: None, dst } => {
            let ret = unsafe {
                IcmpSendEcho2(
                    ICMP_HANDLE,
                    NULL,             // Event
                    callback_fn as _, // ApcRoutine,
                    arcptr as _,      // ApcContext,
                    mem::transmute(dst),
                    job.data_ptr,
                    job.data_len,
                    &mut ip_opts,
                    job.reply_ptr,
                    job.reply_len,
                    job.timeout,
                )
            };
            after_send(ret, arcptr);
        }
        V6 { src, dst } => {
            let mut src = SOCKADDR_IN6 {
                sin6_family: AF_INET6 as _,
                sin6_addr: unsafe {
                    #[allow(clippy::or_fun_call)] // Really clippy... it's a const fn...
                    mem::transmute(src.unwrap_or(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0)))
                },
                ..Default::default()
            };
            let mut dst = SOCKADDR_IN6 {
                sin6_family: AF_INET6 as _,
                sin6_addr: unsafe { mem::transmute(dst) },
                ..Default::default()
            };
            let ret = unsafe {
                Icmp6SendEcho2(
                    ICMP6_HANDLE,
                    NULL,             // Event
                    callback_fn as _, // ApcRoutine
                    arcptr as _,      // ApcContext
                    &mut src,
                    &mut dst,
                    job.data_ptr,
                    job.data_len,
                    &mut ip_opts,
                    job.reply_ptr,
                    job.reply_len,
                    job.timeout,
                )
            };
            after_send(ret, arcptr);
        }
    }
    true
}

extern "system" fn callback_fn(
    state_arc: *const Mutex<State>,
    _io_status_block: *mut VOID,
    _rsvd: ULONG,
) {
    let state_arc = unsafe { Arc::from_raw(state_arc) };
    let mut lock = state_arc.lock().unwrap();
    let state = replace(&mut *lock, State::Invalid);
    match state {
        State::Unpolled(buf) => *lock = State::Ready(buf),
        State::Polled(buf, waker) => {
            *lock = State::Ready(buf);
            waker.wake();
        }
        _ => {} // Leave state as Invalid, pushes panic out of async thread
    }
}

impl Future for PingFuture {
    type Output = AsyncResult;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut lock = self.state.lock().unwrap();
        let state = replace(&mut *lock, State::Invalid);
        match state {
            State::Unpolled(buf) | State::Polled(buf, _) => {
                *lock = State::Polled(buf, cx.waker().clone());
                Poll::Pending
            }
            State::Ready(mut buf) => {
                drop(lock);
                let ret = unsafe {
                    match self.kind {
                        IpKind::V4 => IcmpParseReplies(buf.reply_data_ptr(), buf.reply_data_len()),
                        IpKind::V6 => Icmp6ParseReplies(buf.reply_data_ptr(), buf.reply_data_len()),
                    }
                };
                let result = if ret == 0 {
                    Err(Error::from_lasterror())
                } else {
                    let (status, rtt) = match self.kind {
                        IpKind::V4 => {
                            #[cfg(target_pointer_width = "32")]
                            let reply = buf.as_echo_reply().unwrap();
                            #[cfg(target_pointer_width = "64")]
                            let reply = buf.as_echo_reply32().unwrap();

                            let (status, rtt) = (reply.Status, reply.RoundTripTime);
                            buf.set_filled4();
                            (status, rtt)
                        }
                        IpKind::V6 => {
                            let reply = buf.as_echo_reply6().unwrap();
                            let (status, rtt) = (reply.Status, reply.RoundTripTime as u32);
                            buf.set_filled6();
                            (status, rtt)
                        }
                    };
                    if status == IP_SUCCESS {
                        Ok(rtt)
                    } else {
                        Err(Error::from_iperror(status))
                    }
                };
                Poll::Ready(AsyncResult {
                    result,
                    buffer: buf,
                })
            }
            State::Failed(buf, err) => {
                drop(lock);
                Poll::Ready(AsyncResult {
                    result: Err(Error::from_winerror(err)),
                    buffer: buf,
                })
            }
            State::FailedAsyncSend(_buf, err) => unreachable!(
                "Failed to send async. Expected return of 0, got {} instead.",
                err
            ),
            State::Invalid => unreachable!(),
        }
    }
}
