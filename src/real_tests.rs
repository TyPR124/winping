// Tests that rely on having at least one internet-connected network interface

use crate::*;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use crate::tests::{get_v4_pinger, get_v6_pinger, BOGON4};

const GOOGLE_DNS_A_V4: Ipv4Addr = Ipv4Addr::new(8, 8, 8, 8);
const GOOGLE_DNS_B_V4: Ipv4Addr = Ipv4Addr::new(8, 8, 4, 4);

const GOOGLE_DNS_A_V6: Ipv6Addr = Ipv6Addr::new(0x2001, 0x4860, 0x4860, 0, 0, 0, 0, 0x8888);
const GOOGLE_DNS_B_V6: Ipv6Addr = Ipv6Addr::new(0x2001, 0x4860, 0x4860, 0, 0, 0, 0, 0x8844);

#[cfg(feature = "real-tests-v4")]
#[test]
fn send4_google_dns() {
    let mut buf = Buffer::new();
    // Google truncates return data, so only send 64 bytes
    for x in 0..=63 {
        buf.request_data.push(x)
    }
    let pinger = get_v4_pinger();
    let res = pinger.send4(GOOGLE_DNS_A_V4, &mut buf);
    assert!(res.is_ok());
    assert_eq!(buf.reply_data(), &buf.request_data[..]);
    assert_eq!(buf.responding_ip(), Some(IpAddr::V4(GOOGLE_DNS_A_V4)));
    let res = pinger.send4(GOOGLE_DNS_B_V4, &mut buf);
    assert!(res.is_ok());
    assert_eq!(buf.reply_data(), &buf.request_data[..]);
    assert_eq!(buf.responding_ip(), Some(IpAddr::V4(GOOGLE_DNS_B_V4)));
}

#[cfg(feature = "real-tests-v4")]
#[test]
fn async_send4_google_dns() {
    let mut bufa = Buffer::new();
    let mut bufb = Buffer::new();
    // Google truncates return data, so only send 64 bytes
    for x in 0..=63 {
        bufa.request_data.push(x);
        bufb.request_data.push(x);
    }
    let pinger = AsyncPinger::new();
    let [AsyncResult {
        result: resa,
        buffer: bufa,
    }, AsyncResult {
        result: resb,
        buffer: bufb,
    }] = futures::executor::block_on(async {
        let a = pinger.send4(GOOGLE_DNS_A_V4, bufa);
        let b = pinger.send4(GOOGLE_DNS_B_V4, bufb);
        [a.await, b.await]
    });
    assert!(resa.is_ok());
    assert_eq!(bufa.reply_data(), &bufa.request_data[..]);
    assert_eq!(bufa.responding_ip(), Some(IpAddr::V4(GOOGLE_DNS_A_V4)));

    assert!(resb.is_ok());
    assert_eq!(bufb.reply_data(), &bufb.request_data[..]);
    assert_eq!(bufb.responding_ip(), Some(IpAddr::V4(GOOGLE_DNS_B_V4)));
}

#[cfg(feature = "real-tests-v6")]
#[test]
fn send6_google_dns() {
    let mut buf = Buffer::new();
    // Google truncates return data, so only send 64 bytes
    for x in 0..=63 {
        buf.request_data.push(x)
    }
    let pinger = get_v6_pinger();
    let res = pinger.send6(GOOGLE_DNS_A_V6, &mut buf);
    assert!(res.is_ok());
    assert_eq!(buf.reply_data(), &buf.request_data[..]);
    assert_eq!(buf.responding_ip(), Some(IpAddr::V6(GOOGLE_DNS_A_V6)));
    let res = pinger.send6(GOOGLE_DNS_B_V6, &mut buf);
    assert!(res.is_ok());
    assert_eq!(buf.reply_data(), &buf.request_data[..]);
    assert_eq!(buf.responding_ip(), Some(IpAddr::V6(GOOGLE_DNS_A_V6)));
}

#[cfg(feature = "real-tests-v6")]
#[test]
fn async_send6_google_dns() {
    let mut bufa = Buffer::new();
    let mut bufb = Buffer::new();
    // Google truncates return data, so only send 64 bytes
    for x in 0..=63 {
        bufa.request_data.push(x);
        bufb.request_data.push(x);
    }
    let pinger = AsyncPinger::new();
    let [AsyncResult {
        result: resa,
        buffer: bufa,
    }, AsyncResult {
        result: resb,
        buffer: bufb,
    }] = futures::executor::block_on(async {
        let a = pinger.send6(GOOGLE_DNS_A_V6, bufa);
        let b = pinger.send6(GOOGLE_DNS_B_V6, bufb);
        [a.await, b.await]
    });
    assert!(resa.is_ok());
    assert_eq!(bufa.reply_data(), &bufa.request_data[..]);
    assert_eq!(bufa.responding_ip(), Some(IpAddr::V6(GOOGLE_DNS_A_V6)));

    assert!(resb.is_ok());
    assert_eq!(bufb.reply_data(), &bufb.request_data[..]);
    assert_eq!(bufb.responding_ip(), Some(IpAddr::V6(GOOGLE_DNS_B_V6)));
}

#[cfg(feature = "real-tests-v4")]
#[test]
fn send4_timeout() {
    let mut buf = Buffer::new();
    let pinger = get_v4_pinger();
    let res = pinger.send4(BOGON4, &mut buf);
    assert_eq!(Err(Error::Timeout), res);
}
