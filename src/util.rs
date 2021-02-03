use winapi::shared::in6addr::in6_addr;

use std::net::{Ipv4Addr, Ipv6Addr};
/// Converts a Rust IPv4 to a Windows IPv4
pub(crate) fn rip_to_wip(ip: Ipv4Addr) -> u32 {
    u32::from_ne_bytes(ip.octets())
}
/// Converts a Windows IPv4 to a Rust IPv4
pub(crate) fn wip_to_rip(ip: u32) -> Ipv4Addr {
    Ipv4Addr::from(ip.to_ne_bytes())
}
/// Converts a Rust IPv6 to a Windows IPv6
pub(crate) fn rip6_to_wip6(ip: Ipv6Addr) -> in6_addr {
    // Unsafe can't be avoided when creating an in6_addr,
    // so might as well just transmute it.
    unsafe { std::mem::transmute(ip) }
}
/// Converts a Windows IPv6 to a Rust IPv6
#[allow(clippy::many_single_char_names)]
pub(crate) fn wip6_to_rip6(ip: [u16; 8]) -> Ipv6Addr {
    let [a, b, c, d, e, f, g, h] = ip;
    Ipv6Addr::new(
        u16::from_be(a),
        u16::from_be(b),
        u16::from_be(c),
        u16::from_be(d),
        u16::from_be(e),
        u16::from_be(f),
        u16::from_be(g),
        u16::from_be(h),
    )
}

#[test]
#[allow(clippy::many_single_char_names)]
fn ip_conv_is_correct() {
    let localhost_u32be: u32 = 0x7f000001u32.to_be();
    assert_eq!(localhost_u32be, rip_to_wip(Ipv4Addr::LOCALHOST));
    assert_eq!(Ipv4Addr::LOCALHOST, wip_to_rip(localhost_u32be));

    let localhost6_segments_be = [0, 0, 0, 0, 0, 0, 0, 1u16.to_be()];
    assert_eq!(localhost6_segments_be, *unsafe {
        rip6_to_wip6(Ipv6Addr::LOCALHOST).u.Word()
    });
    assert_eq!(Ipv6Addr::LOCALHOST, wip6_to_rip6(localhost6_segments_be));
}
