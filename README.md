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
`coil-service` is a cross-platform, distributed messaging service that uses a peer-to-peer model to facilitate reliable communications. It offers useful abstractions over low-level networking, providing a foundation for distributed applications.

## Status

`coil-service` is **absolutely not** production ready and is still considered to be "**proof of concept**" status. Do not, under any circumstances, use this software in a production environment. 

## Design decisions

`coil-service` is built on top of [`libp2p`][libp2p], using the [`tokio`][tokio] multi-threaded asynchronous runtime. Ideally, I would like to also provide support for the single-threaded runtime.

## Goals

### Cross-platform support
Ideally, this service will run on as many devices as possible. This project is being developed by one person (myself), and I will do my best to support as many platforms as possible.

The following targets are guaranteed to be fully supported at all times:
  - Windows
    - `x86_64-pc-windows-msvc`
  - MacOS
    - `x86_64-apple-darwin`
  - Linux
    - `x86_64-unknown-linux-gnu`
    
**NOTE**: Support for devices with `aarch64` chip sets (i.e. MacOS M1, Raspberry Pi, etc.) will be added to this list if the work required is deemed feasible. If you would like to help, [contributions][contributing] are welcome.

## Project Setup

### Development

#### Rust Toolchain

This software is currently being developed using the Rust `nightly` [toolchain][project-rust-toolchain].

The presence of the `rust-toolchain.toml` file in the root of the project should automatically install the correct toolchain for you upon first build.
If it does not, you can manually install the toolchain with the following command:

```shell
rustup toolchain install nightly --allow-downgrade --profile default --component cargo clippy llvm-tools-preview rust-src
```

I made this choice because I prefer the improved developer experience offered by the `nightly` toolchain.

The 

This is not permanent, and will change when a PoC and/or MVP is complete. At that stage, work on the MVP will begin, and the project will move to using Rust `stable` toolchain.


#### Dependencies

Setup instructions for each supported platform can be found in the section below:

##### Windows
- [lld linker][lld-linker]
- [Clang compiler frontend][clang]
  - These components should already be installed from `MSVC`.

Install the cargo binaries and rustup components:
```shell
cargo install --force cargo-binutils
rustup component add llvm-tools-preview
```

##### MacOS
- [zld linker][zld]

Install the required linker:
```shell
brew install michaeleisel/zld/zld
```

##### Linux
- [lld linker][lld-linker]
- [Clang compiler frontend][clang]
Install the linker and compiler frontend:
```shell
sudo apt-get install lld clang
```

## License

This project is licensed under the [MIT license][license].

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in `coil-service` by you, shall be licensed as MIT, without any additional terms or conditions.

<!-- Links -->

<!-- crate docs -->
[docs]: https://docs.rs/coil-service
<!-- dependencies -->
[tokio]: https://crates.io/crates/tokio
[libp2p]: https://crates.io/crates/libp2p
<!-- Linker -->
[clang]: https://clang.llvm.org/
[lld-linker]: https://lld.llvm.org/
[zld]: https://github.com/michaeleisel/zld
<!-- Repository -->
[contributing]: ./CONTRIBUTING.md
[license]: ./LICENSE
[project-rust-toolchain]: ./rust-toolchain.toml