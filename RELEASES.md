# winping release notes

## [0.10.0](https://crates.io/crates/winping/0.10.0)

* Now allows customizing the async buffer size. This is described in the docs for `set_async_buffer_size`
* Removed the "no-async" feature. This is replaced with an "async" feature that is enabled by default. If you were previously using the "no-async" feature, you can accomplish the ssame thing using an empty feature-set, ie `winping = { version = "0.10", features = [] }`
* Added async tests to the "real-tests" feature (this "feature" is used for testing pings to real-world IP addresses, namely Google's DNS servers. The tests outside this feature only test with loopbacks and reserved IPs)
* Fixed typo in Display/Debug impl of `Error::HostUnreachable` (Thanks @denisbrodbeck)

## [0.9.3](https://crates.io/crates/winping/0.9.3)

* Updated dependencies (in particular, quote v1.0.2 was yanked, now on quote v1.0.3)

## [0.9.2](https://crates.io/crates/winping/0.9.2)

NOTE: Version Yanked due to a yanked depencency

* Added `std::error::Error` impls for  `winping::Error` and `winping::CreateError`
* Added `std::fmt::Display` impl for `winping::CreateError`
