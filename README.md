# winping

[![Tests (MSVC)](https://github.com/TyPR124/winping/workflows/Tests%20MSVC/badge.svg)](https://github.com/TyPR124/winping/actions?query=workflow%3A%22Tests%20MSVC%22)
[![Tests (GNU)](https://github.com/TyPR124/winping/workflows/Tests%20GNU/badge.svg)](https://github.com/TyPR124/winping/actions?query=workflow%3A%22Tests%20GNU%22)
[![crates.io](https://meritbadge.herokuapp.com/winping)](https://crates.io/crates/winping)
[![docs.rs](https://docs.rs/winping/badge.svg)](https://docs.rs/winping)
[![MIT Licensed](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE-MIT)
[![Apache2 Licensed](https://img.shields.io/badge/license-Apache2-blue.svg)](./LICENSE-APACHE)

An easy-to-use ICMP Echo library for Windows. Does not require elevated permissions. Has async support.

## Usage

To use in your own project, simply add `winping = "0.1"` to your dependencies in your Cargo.toml file. See [docs.rs](https://docs.rs/winping) for examples and documentation.

## OS Support

As of now, this crate is only tested on Windows 10. I will update this when I have been able to test on more systems. This crate will almost certainly not work on Windows XP. I suspect it will work for Vista but have no plans to officially support anything before Windows 7. Automated tests run on Windows Server 2019 with both MSVC and GNU compilers.

## Contributions

Contributions of all kinds are welcome. File a GitHub issue if you find a bug or think something can be improved. If you wish to contribute code, please run `cargo fmt` and `cargo clippy`. You should probably also run `cargo test`.
