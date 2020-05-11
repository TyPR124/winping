# winping

[![Tests (MSVC)](https://github.com/TyPR124/winping/workflows/Tests%20MSVC/badge.svg)](https://github.com/TyPR124/winping/actions?query=workflow%3A%22Tests%20MSVC%22)
[![Tests (GNU)](https://github.com/TyPR124/winping/workflows/Tests%20GNU/badge.svg)](https://github.com/TyPR124/winping/actions?query=workflow%3A%22Tests%20GNU%22)
[![crates.io](https://meritbadge.herokuapp.com/winping)](https://crates.io/crates/winping)
[![docs.rs](https://docs.rs/winping/badge.svg)](https://docs.rs/winping)
[![MIT Licensed](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE-MIT)
[![Apache2 Licensed](https://img.shields.io/badge/license-Apache2-blue.svg)](./LICENSE-APACHE)

An easy-to-use ICMP Echo library for Windows. Does not require elevated permissions. Has async support.

## Usage

To use in your own project, simply add `winping = "0.10"` to your dependencies in your Cargo.toml file. See [docs.rs](https://docs.rs/winping) for examples and documentation.

## OS Compatability

This crate has been tested on Windows 7, Windows 8.1, and Windows 10.

This crate intends to support Windows 7 through Windows 10, including both desktop and server variants.

This crate is very unlikely to work with Windows XP. It may work with Vista, however Vista will not be supported.

## Contributions

Contributions of all kinds are welcome. File a GitHub issue if you find a problem, bug, or think something can be improved. If you wish to contribute code, please run `cargo fmt` and `cargo clippy`. You should probably also run `cargo test`.
