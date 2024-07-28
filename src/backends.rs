// SPDX-License-Identifier: Apache-2.0 OR MIT
// SPDX-FileCopyrightText: © 2024 Google
// Ported from libtypec (Rajaram Regupathy <rajaram.regupathy@gmail.com>)

//! The OS specific backends

#[cfg(target_os = "linux")]
pub mod sysfs;
#[cfg(target_os = "linux")]
pub mod ucsi_debugfs;
