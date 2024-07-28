// SPDX-License-Identifier: Apache-2.0 OR MIT

//! libtypec-rs is a library that aims to provide a generic interface
//! abstracting all platform complexity for user space to develop tools for
//! efficient USB-C port management and efficient diagnostic and debugging
//! tools to debug of system issues around USB-C/USB PD topology.
//!
//! The data structures and interface APIs are based on USB Type-C® Connector
//! System Software Interface (UCSI) Specification for the most part.

// Note: this library is written in Rust, but one of its goals is to provide a C
// FFI. Cbindgen is a tool that generates C bindings for Rust code. It is used
// to generate a compatible header file. It does *not* understand namespacing,
// since namespaces are not officially in the C language. This means that the
// Rust types used throughout the library retain a (more verbose) full name in
// spite of the module they are declared in.

#![cfg_attr(feature = "backtrace", feature(error_generic_member_access))]

use std::io::Cursor;

use bitstream_io::LittleEndian;
use pd::Message;
use pd::MessageRecipient;
use pd::MessageResponseType;
use pd::Pdo;
use ucsi::AlternateMode;
use ucsi::CableProperty;
use ucsi::Capability;
use ucsi::ConnectorCapability;
use ucsi::ConnectorStatus;
use ucsi::GetAlternateModesRecipient;
use ucsi::PdoSourceCapabilitiesType;
use ucsi::PdoType;

pub mod backends;
pub mod pd;
pub mod typec;
pub mod ucsi;
pub mod vdo;

pub type BitWriter<'a> = bitstream_io::BitWriter<Cursor<&'a mut [u8]>, LittleEndian>;
pub type BitReader<'a> = bitstream_io::BitReader<Cursor<&'a [u8]>, LittleEndian>;
pub type Result<T> = std::result::Result<T, crate::Error>;

/// Wrap a bitflags! invocation.
///
/// cbindgen does not support both parse.expand *and* bitflags=true, because it
/// will also expand the bitflags macro. We have our own wrapper instead.
///
/// This will give Rust users a nice, idiomatic API with the bitflags crate,
/// while giving C users a typedef and #defines.
///
/// A module is used to gate the constants to pub(crate).
#[macro_export]
macro_rules! bitflags_wrapper {
    (
        $prefix:ident,
        $(#[$outer:meta])*
        $vis:vis struct $name:ident: $t:ty {
            $(
                const $flag:ident = $value:expr;
            )*
        }) => {
        bitflags::bitflags! {
            $(#[$outer])*
            /// cbindgen:ignore
            $vis struct $name: $t {
                $(
                    const $flag = $value;
                )*
            }
        }

        paste::paste! {
            #[repr(transparent)]
            $(#[$outer])*
            #[cfg(feature="c_api")]
            pub(crate) struct [< $prefix $name >] {
                bits: $t,
            }

            #[cfg(feature="c_api")]
            pub(crate) mod [< $prefix:snake:lower _ $name:snake:lower _ flags >] {
                $(
                    #[allow(dead_code)]
                    pub const [< $prefix:snake:upper _ $name:snake:upper _ $flag  >]: $t = $value;
                )*
            }

            #[cfg(feature="c_api")]
            impl From<$name> for [< $prefix $name >] {
                fn from(original: $name) -> Self {
                    Self { bits: original.bits() }
                }
            }

            #[cfg(feature="c_api")]
            impl From<[< $prefix $name >]> for $name {
                fn from(prefixed: [< $prefix $name >]) -> Self {
                    Self::from_bits_truncate(prefixed.bits)
                }
            }
        }
    };
}

// A trait that abstracts the platform-specific backend.
pub trait OsBackend {
    fn capabilities(&mut self) -> Result<Capability>;

    fn connector_capabilties(&mut self, connector_nr: usize) -> Result<ConnectorCapability>;

    fn alternate_modes(
        &mut self,
        recipient: GetAlternateModesRecipient,
        connector_nr: usize,
    ) -> Result<Vec<AlternateMode>>;

    fn cable_properties(&mut self, connector_nr: usize) -> Result<CableProperty>;

    fn connector_status(&mut self, connector_nr: usize) -> Result<ConnectorStatus>;

    fn pd_message(
        &mut self,
        connector_nr: usize,
        recipient: MessageRecipient,
        response_type: MessageResponseType,
    ) -> Result<Message>;

    #[allow(clippy::too_many_arguments)]
    fn pdos(
        &mut self,
        connector_nr: usize,
        partner_pdo: bool,
        pdo_offset: u32,
        nr_pdos: usize,
        pdo_type: PdoType,
        source_capabilities_type: PdoSourceCapabilitiesType,
        revision: BcdWrapper,
    ) -> Result<Vec<Pdo>>;
}

/// A trait for serializing an object to a byte stream.
///
/// This is used to write an object to a byte array when needed.
pub trait ToBytes {
    /// Serializes the object to a byte stream.
    fn to_bytes(&self, bit_writer: &mut BitWriter) -> Result<()>;
}

/// A trait for deserializing an object from a byte stream.
///
/// This is used to read an object from a byte array when needed.
pub trait FromBytes {
    /// Deserializes the object from a byte stream.
    fn from_bytes(bit_reader: &mut BitReader) -> Result<Self>
    where
        Self: Sized;
}

#[repr(transparent)]
#[derive(Clone, Copy, Default, PartialEq)]
/// A wrapper that can pretty-print the underlying BCD value.
pub struct BcdWrapper(u32);

impl std::fmt::Debug for BcdWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:x}.{:x}", (self.0 >> 8) & 0xff, self.0 & 0xff)
    }
}

#[derive(thiserror::Error)]
/// An error type for the library.
pub enum Error {
    #[error("{source}")]
    NixError {
        #[from]
        source: nix::Error,
        #[cfg(feature = "backtrace")]
        backtrace: std::backtrace::Backtrace,
    },
    #[error("{source}")]
    IoError {
        #[from]
        source: std::io::Error,
        #[cfg(feature = "backtrace")]
        backtrace: std::backtrace::Backtrace,
    },
    #[error("This operation is not supported")]
    NotSupported {
        #[cfg(feature = "backtrace")]
        backtrace: std::backtrace::Backtrace,
    },
    #[error("Could not parse field {field} with value {value}")]
    ParseError {
        field: String,
        value: u32,
        #[cfg(feature = "backtrace")]
        backtrace: std::backtrace::Backtrace,
    },
    #[error("Could not parse field {field} with value {value}")]
    ParseStringError {
        field: String,
        value: String,
        #[cfg(feature = "backtrace")]
        backtrace: std::backtrace::Backtrace,
    },
    #[error("{source}")]
    Utf8Error {
        #[from]
        source: std::str::Utf8Error,
        #[cfg(feature = "backtrace")]
        backtrace: std::backtrace::Backtrace,
    },
    #[error("Timed out waiting for a response")]
    TimeoutError {
        #[cfg(feature = "backtrace")]
        backtrace: std::backtrace::Backtrace,
    },
    #[error("This USB revision is not supported: {revision:?}")]
    UnsupportedUsbRevision {
        revision: BcdWrapper,
        #[cfg(feature = "backtrace")]
        backtrace: std::backtrace::Backtrace,
    },
    #[error("{source}")]
    NulError {
        #[from]
        source: std::ffi::NulError,
        #[cfg(feature = "backtrace")]
        backtrace: std::backtrace::Backtrace,
    },
    #[error("{source}")]
    WalkdirError {
        #[from]
        source: walkdir::Error,
        #[cfg(feature = "backtrace")]
        backtrace: std::backtrace::Backtrace,
    },
}

// Some boilerplate to make the backtraces more readable
impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NixError {
                source,
                #[cfg(feature = "backtrace")]
                backtrace,
            } => {
                f.debug_struct("NixError")
                    .field("source", source)
                    .finish()?;

                #[cfg(feature = "backtrace")]
                write!(f, "\n\nerror stack backtrace:\n{}", backtrace)
            }
            Self::IoError {
                source,
                #[cfg(feature = "backtrace")]
                backtrace,
            } => {
                f.debug_struct("IoError").field("source", source).finish()?;

                #[cfg(feature = "backtrace")]
                write!(f, "\n\nerror stack backtrace:\n{}", backtrace)
            }
            Self::Utf8Error {
                source,
                #[cfg(feature = "backtrace")]
                backtrace,
            } => {
                f.debug_struct("Utf8Error")
                    .field("source", source)
                    .finish()?;

                #[cfg(feature = "backtrace")]
                write!(f, "\n\nerror stack backtrace:\n{}", backtrace)
            }
            Self::NulError {
                source,
                #[cfg(feature = "backtrace")]
                backtrace,
            } => {
                f.debug_struct("NulError")
                    .field("source", source)
                    .finish()?;

                #[cfg(feature = "backtrace")]
                write!(f, "\n\nerror stack backtrace:\n{}", backtrace)
            }
            Self::WalkdirError {
                source,
                #[cfg(feature = "backtrace")]
                backtrace,
            } => {
                f.debug_struct("WalkdirError")
                    .field("source", source)
                    .finish()?;

                #[cfg(feature = "backtrace")]
                write!(f, "\n\nerror stack backtrace:\n{}", backtrace)
            }
            Self::NotSupported {
                #[cfg(feature = "backtrace")]
                backtrace,
            } => {
                f.debug_struct("NotSupported").finish()?;

                #[cfg(feature = "backtrace")]
                write!(f, "\n\nerror stack backtrace:\n{}", backtrace)
            }
            Self::TimeoutError {
                #[cfg(feature = "backtrace")]
                backtrace,
            } => {
                f.debug_struct("TimeoutError").finish()?;

                #[cfg(feature = "backtrace")]
                write!(f, "\n\nerror stack backtrace:\n{}", backtrace)
            }
            Self::ParseError {
                field,
                value,
                #[cfg(feature = "backtrace")]
                backtrace,
            } => {
                f.debug_struct("ParseError")
                    .field("field", field)
                    .field("value", value)
                    .finish()?;

                #[cfg(feature = "backtrace")]
                write!(f, "\n\nerror stack backtrace:\n{}", backtrace)
            }
            Self::ParseStringError {
                field,
                value,
                #[cfg(feature = "backtrace")]
                backtrace,
            } => {
                f.debug_struct("ParseStringError")
                    .field("field", field)
                    .field("value", value)
                    .finish()?;

                #[cfg(feature = "backtrace")]
                write!(f, "\n\nerror stack backtrace:\n{}", backtrace)
            }
            Self::UnsupportedUsbRevision {
                revision,
                #[cfg(feature = "backtrace")]
                backtrace,
            } => {
                f.debug_struct("UnsupportedUsbRevision")
                    .field("revision", revision)
                    .finish()?;

                #[cfg(feature = "backtrace")]
                write!(f, "\n\nerror stack backtrace:\n{}", backtrace)
            }
        }
        #[cfg(not(feature = "backtrace"))]
        Ok(())
    }
}

#[repr(transparent)]
pub struct CError(pub std::ffi::c_int);

impl From<Error> for CError {
    /// Converts an Error to a C error number
    fn from(err: Error) -> Self {
        match err {
            Error::NixError { source, .. } => CError(source as i32),
            Error::IoError { source, .. } => {
                CError(source.raw_os_error().unwrap_or(nix::libc::EIO))
            }
            Error::NotSupported { .. } => CError(nix::libc::EOPNOTSUPP),
            Error::ParseError { .. }
            | Error::Utf8Error { .. }
            | Error::NulError { .. }
            | Error::WalkdirError { .. }
            | Error::ParseStringError { .. } => CError(nix::libc::EIO),
            Error::TimeoutError { .. } => CError(nix::libc::ETIMEDOUT),
            Error::UnsupportedUsbRevision { .. } => CError(nix::libc::ENOTSUP),
        }
    }
}

fn is_chrome_os() -> bool {
    #[cfg(target_os = "linux")]
    match nix::sys::utsname::uname() {
        Ok(uname) => uname.sysname().to_string_lossy().contains("chrome"),
        Err(_) => false,
    }
    #[cfg(not(target_os = "linux"))]
    false
}

#[repr(transparent)]
#[derive(Clone, Copy, Default, PartialEq)]
/// A wrapper that can pretty-print the underlying millivolt value.
pub struct Millivolt(pub u32);

impl std::fmt::Debug for Millivolt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}mV", self.0)
    }
}

impl From<u32> for Millivolt {
    fn from(val: u32) -> Self {
        Millivolt(val)
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Default, PartialEq)]
/// A wrapper that can pretty-print the underlying milliamp value.
pub struct Milliamp(pub u32);

impl std::fmt::Debug for Milliamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}mA", self.0)
    }
}

impl From<u32> for Milliamp {
    fn from(val: u32) -> Self {
        Milliamp(val)
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Default, PartialEq)]
/// A wrapper that can pretty-print the underlying milliwatt value.
pub struct Milliwatt(pub u32);

impl std::fmt::Debug for Milliwatt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}mW", self.0)
    }
}

impl From<u32> for Milliwatt {
    fn from(val: u32) -> Self {
        Milliwatt(val)
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Default, PartialEq, Eq)]
/// A wrapper that can pretty-print the underlying milliohm value.
pub struct Milliohm(pub u32);

impl std::fmt::Debug for Milliohm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}mΩ", self.0)
    }
}

impl From<u32> for Milliohm {
    fn from(val: u32) -> Self {
        Self(val)
    }
}
