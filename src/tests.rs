use crate::*;
use std::net::{Ipv4Addr, Ipv6Addr};

use futures::{
    executor::LocalPool,
    task::{FutureObj, Spawn},
    FutureExt,
};

pub(crate) const LO4: Ipv4Addr = Ipv4Addr::new(127, 0, 0, 1);
pub(crate) const BOGON4: Ipv4Addr = Ipv4Addr::new(198, 18, 0, 1);
pub(crate) const LO6: Ipv6Addr = Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1);

pub(crate) fn get_v4_pinger() -> Pinger {
    match Pinger::new() {
        Ok(ret) | Err(CreateError::NoV6(ret)) => ret,
        _ => panic!("Could not create v4 pinger"),
    }
}
pub(crate) fn get_v6_pinger() -> Pinger {
    match Pinger::new() {
        Ok(ret) | Err(CreateError::NoV4(ret)) => ret,
        _ => panic!("Could not create v6 pinger"),
    }
}

#[test]
fn send4() {
    let mut buf = Buffer::new();
    let pinger = get_v4_pinger();
    let res = pinger.send4(LO4, &mut buf);
    assert!(res.is_ok());
}
#[test]
fn send4_timeout() {
    let mut buf = Buffer::new();
    let pinger = get_v4_pinger();
    let res = pinger.send4(BOGON4, &mut buf);
    assert_eq!(Err(Error::Timeout), res);
}
#[test]
fn send4_from() {
    let mut buf = Buffer::new();
    let pinger = get_v4_pinger();
    let res = pinger.send4_from(LO4, LO4, &mut buf);
    assert!(res.is_ok())
}
#[test]
fn send4_from_unreachable() {
    let mut buf = Buffer::new();
    let pinger = get_v4_pinger();
    let res = pinger.send4_from(LO4, BOGON4, &mut buf);
    assert_eq!(Err(Error::NetUnreachable), res);
}
#[test]
fn send6() {
    let mut buf = Buffer::new();
    let pinger = get_v6_pinger();
    let res = pinger.send6(LO6, &mut buf);
    assert!(res.is_ok());
}

#[test]
fn send6_from() {
    let mut buf = Buffer::new();
    let pinger = get_v6_pinger();
    let res = pinger.send6_from(LO6, LO6, &mut buf);
    assert!(res.is_ok());
}

#[cfg(not(feature = "no_async"))]
#[test]
fn async_send4() {
    let pinger = AsyncPinger::new();

    let mut pool = LocalPool::new();
    let spawner = pool.spawner();
    let dst = LO4;

    for _ in 0..10 {
        let buf = Buffer::new();
        let fut = FutureObj::new(Box::pin(pinger.send4(dst, buf).map(|res| {
            assert!(res.result.is_ok());
        })));
        spawner.spawn_obj(fut).unwrap();
    }
    pool.run();
}
#[cfg(not(feature = "no_async"))]
#[test]
fn async_send6() {
    let pinger = AsyncPinger::new();

    let mut pool = LocalPool::new();
    let spawner = pool.spawner();
    let dst = LO6;

    for _ in 0..10 {
        let buf = Buffer::new();
        let fut = FutureObj::new(Box::pin(pinger.send6(dst, buf).map(|res| {
            assert!(res.result.is_ok());
        })));
        spawner.spawn_obj(fut).unwrap();
    }
    pool.run();
}
#[cfg(not(feature = "no_async"))]
#[test]
fn async_send4_from() {
    let pinger = AsyncPinger::new();

    let mut pool = LocalPool::new();
    let spawner = pool.spawner();
    let src = LO4;
    let dst = LO4;

    for _ in 0..10 {
        let buf = Buffer::new();
        let fut = FutureObj::new(Box::pin(pinger.send4_from(src, dst, buf).map(|res| {
            assert!(res.result.is_ok());
        })));
        spawner.spawn_obj(fut).unwrap();
    }
    pool.run();
}
#[cfg(not(feature = "no_async"))]
#[test]
fn async_send6_from() {
    let pinger = AsyncPinger::new();

    let mut pool = LocalPool::new();
    let spawner = pool.spawner();
    let src = LO6;
    let dst = LO6;

    for _ in 0..10 {
        let buf = Buffer::new();
        let fut = FutureObj::new(Box::pin(pinger.send6_from(src, dst, buf).map(|res| {
            assert!(res.result.is_ok());
        })));
        spawner.spawn_obj(fut).unwrap();
    }
    pool.run();
}

#[test]
fn error_win_display() {
    let e = Error::Other(0);
    let s = format!("{}", e);
    // println!("'{}'", s);
    assert!(s.ends_with("The operation completed successfully."));
}
#[test]
fn error_ip_display() {
    let e = Error::Other(11001);
    let s = format!("{}", e);
    // println!("'{}'", s);
    assert!(s.ends_with("Buffer too small."));
}
