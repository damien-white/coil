<div align="center">
  <h1>COIL</h1>
  <h2>
    <a href="https://crates.io/crates/coil-service" rel="noreferrer noopener">coil-service
    </a>
  </h2>
  <!-- TODO: Add a custom logo instead of the `h1` tag (example on next line) -->
  <!-- <img src="assets/images/logo-raster.png"  alt="project logo"/> -->
  <h3>Cross-platform, distributed messaging service</h3>
  <br/>

[![crates.io](https://img.shields.io/crates/v/coil-service.svg)](https://crates.io/crates/coil-service)
[![docs.rs](https://docs.rs/coil-service/badge.svg)](https://docs.rs/coil-service)
[![ci](https://github.com/peter-donovan/coil-service/workflows/CI/badge.svg)](https://github.com/peter-donovan/coil-service/actions)
[![coverage Status](https://coveralls.io/repos/github/peter-donovan/coil-service/badge.svg)](https://coveralls.io/github/peter-donovan/coil-service)

[Documentation][docs]

</div>

## Description

`coil-service` is a cross-platform, distributed messaging service that uses a peer-to-peer model to facilitate reliable communications. It offers abstractions over low-level networking components provided by [`libp2p`][libp2p] and [`tokio`][tokio]. 

`coil-service` is free, open-source software and may be used in your work, according with the [licensing][license]. I hope that you do find a use for it, or that it inspires you to build something amazing. Either way, this project will remain open-source for as long as the project exists.

## Project Status

### Activity 

`coil-service` is under active development.

### Warning

**Do not**, under any circumstances, use this software in a production environment. If you choose to use this software, or any components of it, within your own software, you do so entirely at your own risk.

## Design decisions

`coil-service` is built on top of [`libp2p`][libp2p], using the [`tokio`][tokio] multi-threaded asynchronous runtime. Ideally, I would like to also provide support for the single-threaded runtime.

## Goals

- Expose intuitive, high-level APIs to allow users to build their own custom networking utilities and protocol stacks
- Avoid protocol ossification (you will never be forced to use `TCP` or `UDP`, etc.)
  - Please note, that for IPC, your choices may be limited, based on your target platform
- Provide cross-platform support for Windows, Linux and MacOS
  - **NOTE**: Target cpu architecture is assumed to be `x86-64`
- Create abstractions over [`libp2p`][libp2p] that allow developers to work at a higher-level
- Provide "escape-hatches" when deemeded necessary, so that developers can fine-tune components for performance, resource efficiency, etc.

### Cross-platform
Ideally, this service will run on as many devices as possible. This project is being developed by one person (myself), and I will do my best to support as many platforms as possible.

I own machines that natively run all three main desktop platforms, but am doing much of the initial development on my
Windows and Linux machines. If support for MacOS (x86-64, not aarch64) lags behind, or a critical feature is missing, or 
not working, please open a ticket! I will do my absolute best to keep support for the main desktop platforms in lockstep.

#### Primary Targets:
These targets will be treated as first-class citizens:
- Windows: `x86_64-pc-windows-msvc`
- Linux: `x86_64-unknown-linux-gnu`
- MacOS: `x86_64-apple-darwin`

#### Secondary Targets
These targets will ideally be added to the list above, but for the time being, support depends on how much time I have, and how simple it is to add support for these platforms:
- Raspberry Pi 4: `aarch64-unknown-linux-gnu`
- MacOS M1: `aarch64-apple-darwin`
- Windows GNU: `x86_64-pc-windows-gnu`
  - This target will be moved to "Primary targets" if there are few to no complications.
- TBA

If you would like to help add support for additional platforms, I would absolutely love your help. Any and all [contributions][contributing] are welcome!

## Non-goals / Anti-goals

- This project has absolutely nothing to do with "blockchain", and it never will.
- 

## Project Setup

### Development

#### Rust Toolchain

This software is currently being developed using the Rust `nightly` [toolchain][project-rust-toolchain].

The presence of the `rust-toolchain.toml` file in the root of the project should automatically install the correct toolchain for you upon first build.
If it does not, you can manually install the toolchain with the following command:

```bash
rustup toolchain install nightly --allow-downgrade --profile default --component cargo clippy llvm-tools-preview rust-src
```

I made this choice because I prefer the improved developer experience offered by the `nightly` toolchain.

The 

This is not permanent, and will change when a PoC and/or MVP is complete. At that stage, work on the MVP will begin, and the project will move to using Rust `stable` toolchain.


#### Dependencies

Setup instructions for each supported platform can be found in the section below:

##### Windows
These tools should be installed already via `Visual Studio Build Tools`. You can also use a package manager, such as [`chocolatey`][chocolatey] or [`scoop`][scoop].
- [`lld` linker][lld-linker]
- [`Clang` compiler frontend][clang]
  - These components should already be installed from `MSVC`
  - *They may need to be manually added to your PATH variable*

Install the cargo binaries and rustup components:
```bash
cargo install --force cargo-binutils
rustup component add llvm-tools-preview
```

##### MacOS
- [`zld` linker][zld]

Install the required linker:
```bash
brew install michaeleisel/zld/zld
```

##### Linux
- [`lld` linker][lld-linker]
- [`Clang` compiler frontend][clang]
Install the linker and compiler frontend:
```bash
sudo apt-get install lld clang
```

## License

This project is licensed under the [MIT license][license].

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in `coil-service` by you, shall be licensed as MIT, without any additional terms or conditions.

<!-- Links section -->

<!-- crate docs -->
[docs]: https://docs.rs/coil
<!-- dependencies -->
[tokio]: https://crates.io/crates/tokio
[libp2p]: https://crates.io/crates/libp2p
<!-- Linker dependencies -->
[clang]: https://clang.llvm.org/
[lld-linker]: https://lld.llvm.org/
[zld]: https://github.com/michaeleisel/zld
<!-- Repository -->
[contributing]: ./CONTRIBUTING.md
[license]: ./LICENSE
[project-rust-toolchain]: ./rust-toolchain.toml
<!-- Windows package managers -->
[chocolatey]: https://chocolatey.org/
[scoop]: https://scoop.sh/