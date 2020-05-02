//! winping - Easy ICMP Echo for Windows, and no elevated rights required!
//!
//! Super basic ping.exe example
//!
//! ```rust
//! use std::net::IpAddr;
//! use winping::{Buffer, Pinger};
//!
//! fn main() {
//!     let dst = std::env::args()
//!         .nth(1)
//!         .unwrap_or(String::from("127.0.0.1"))
//!         .parse::<IpAddr>()
//!         .expect("Could not parse IP Address");
//!
//!     let pinger = Pinger::new().unwrap();
//!     let mut buffer = Buffer::new();
//!     
//!     for _ in 0..4 {
//!         match pinger.send(dst, &mut buffer) {
//!             Ok(rtt) => println!("Response time {} ms.", rtt),
//!             Err(err) => println!("{}.", err),
//!         }
//!     }
//! }
//! ```
//!
#![cfg(any(target_os = "windows", doc))]
#![forbid(unreachable_patterns)]
#![allow(clippy::needless_doctest_main)]

#[cfg(feature = "async")]
mod async_pinger;
mod buffer;
mod error;
mod pinger;

#[cfg(feature = "async")]
pub use async_pinger::{AsyncPinger, AsyncResult, PingFuture, ASYNC_BUFFER_SIZE};
pub use buffer::Buffer;
pub use error::Error;
pub use pinger::{CreateError, IpPair, Pinger};

#[cfg(test)]
mod tests;

#[cfg(all(test, any(feature = "real-tests-v4", feature = "real-tests-v6")))]
mod real_tests;
