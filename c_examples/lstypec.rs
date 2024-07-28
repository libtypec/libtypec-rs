// SPDX-License-Identifier: Apache-2.0 OR MIT
// SPDX-FileCopyrightText: © 2024 Google
// Ported from libtypec (Rajaram Regupathy <rajaram.regupathy@gmail.com>)

//! Just a wrapper to run examples/ucsi.c through cargo.

use argh::FromArgs;
use libtypec_rs::typec::OsBackends;

#[link(name = "c_examples_lstypec")]
extern "C" {
    fn c_example_lstypec(backend: u32) -> std::ffi::c_int;
}

// Bring the library into scope so that its symbols become available to the
// linker when linking the C code.
extern crate libtypec_rs;

#[derive(FromArgs)]
/// Run the C example for lstypec. This is meant as a documentation for the use
/// of the C API.
struct Args {
    /// the backend to use in the example. Defaults to sysfs.
    #[argh(option)]
    backend: Option<libtypec_rs::typec::OsBackends>,
}

fn main() {
    unsafe {
        let args: Args = argh::from_env();
        let backend = if let Some(backend) = args.backend {
            backend as u32
        } else {
            OsBackends::Sysfs as u32
        };

        c_example_lstypec(backend);
    }
}
