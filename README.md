# libtypec-rs

A library that aims to provide a generic interface abstracting all platform
complexity for user space to develop tools for efficient USB-C port management
and efficient diagnostic and debugging tools to debug of system issues around
USB-C/USB PD topology. The data structures and interface APIs are based on USB
Type-C® Connector System Software Interface (UCSI) Specification for the most
part.

This library compiles to both a `rlib` (to be used in other Rust projects through
Cargo) and as dynamically linked library that can be installed on the system
through a `.deb` package. A `pkg-config` file is provided to aid in linking
against `libtypec-rs`.

Compiling `libtypec-rs` requires the `nightly` toolchain to expand macros before
`libtypec-rs.h` can be generated. The code does not depend on `nightly` except
when compiling with the optional `backtrace` feature for debugging purposes.

## Backends
* `sysfs` - extract information using sysfs.
* `linux_ucsi` - extract information using the Linux UCSI driver debugfs interface.

## Features
* `c_api` - generate a C header file (.h) to be used when linking against `libtypec-rs.so`.
* `backtrace` - generate a backtrace on errors for debugging purposes. Requires nightly.

## Binaries
`lstypec` - list the USB-C information in the system.

Run with:

```
cargo run --bin lstypec
```


`typecstatus` - check the status of the TypeC ports.

Run with:

```
cargo run --bin typecstatus
```

To debug errors, enable the `backtrace` feature and set the `RUST_BACKTRACE` variable:

```
RUST_BACKTRACE=1 cargo run --bin lstypec --features backtrace
```


## Dependencies
- `pkg-config`
-  `libudev-dev`


## Building
Run `cargo build`

## Packaging

A `.deb` package can be automatically generated for this library using `cargo-deb`.

```
# If you do not have cargo-deb installed:
cargo install cargo-deb

# Compile a release version of the library with the C api enabled.

cargo build --features c_api --release

# Then
./cargo-deb
```

This will generate a `.deb` package in `target/debian` whose contents are:
```
└── usr
    ├── bin
    │   ├── lstypec
    │   └── typecstatus
    ├── include
    │   └── libtypec-rs.h
    ├── lib
    │   ├── liblibtypec_rs.so
    │   └── pkgconfig
    │       └── libtypec_rs.pc
    └── share
        └── doc
            └── libtypec-rs
                └── copyright
```

See the c_examples crate as a guide to use the C API provided by this crate.
