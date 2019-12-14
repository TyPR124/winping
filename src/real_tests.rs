use crate::*;
use std::net::{Ipv4Addr, Ipv6Addr};

use crate::tests::{get_v4_pinger, get_v6_pinger, BOGON4};

const GOOGLE_DNS_A_V4: Ipv4Addr = Ipv4Addr::new(8, 8, 8, 8);
const GOOGLE_DNS_B_V4: Ipv4Addr = Ipv4Addr::new(8, 8, 4, 4);

const GOOGLE_DNS_A_V6: Ipv6Addr = Ipv6Addr::new(0x2001, 0x4860, 0x4860, 0, 0, 0, 0, 0x8888);
const GOOGLE_DNS_B_V6: Ipv6Addr = Ipv6Addr::new(0x2001, 0x4860, 0x4860, 0, 0, 0, 0, 0x8844);

#[test]
fn send4_google_dns() {
    let mut buf = Buffer::new();
    let pinger = get_v4_pinger();
    let res = pinger.send4(GOOGLE_DNS_A_V4, &mut buf);
    assert!(res.is_ok());
    let res = pinger.send4(GOOGLE_DNS_B_V4, &mut buf);
    assert!(res.is_ok());
}

#[test]
fn send6_google_dns() {
    let mut buf = Buffer::new();
    let pinger = get_v6_pinger();
    let res = pinger.send6(GOOGLE_DNS_A_V6, &mut buf);
    assert!(res.is_ok());
    let res = pinger.send6(GOOGLE_DNS_B_V6, &mut buf);
    assert!(res.is_ok());
}

#[test]
fn send4_timeout() {
    let mut buf = Buffer::new();
    let pinger = get_v4_pinger();
    let res = pinger.send4(BOGON4, &mut buf);
    assert_eq!(Err(Error::Timeout), res);
}
