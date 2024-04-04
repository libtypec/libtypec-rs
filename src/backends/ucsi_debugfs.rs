// SPDX-License-Identifier: Apache-2.0 OR MIT
// SPDX-FileCopyrightText: © 2024 Google
// Ported from libtypec (Rajaram Regupathy <rajaram.regupathy@gmail.com>)

//! The UCSI backend

use std::io::Cursor;

use crate::pd::Message;
use crate::pd::MessageRecipient;
use crate::pd::MessageResponseType;
use crate::pd::Pdo;
use crate::ucsi::AlternateMode;
use crate::ucsi::CableProperty;
use crate::ucsi::Capability;
use crate::ucsi::Command;
use crate::ucsi::ConnectorCapability;
use crate::ucsi::ConnectorStatus;
use crate::ucsi::GetAlternateModesRecipient;
use crate::ucsi::PdoSourceCapabilitiesType;
use crate::ucsi::PdoType;
use crate::BcdWrapper;
use crate::BitReader;
use crate::Error;
use crate::FromBytes;
use crate::OsBackend;
use crate::Result;
use crate::ToBytes;

/// A mere convenience to check if a response is null.
trait NullResponse {
    fn is_null(&self) -> bool;
}

impl NullResponse for Vec<u8> {
    /// A convenience method to identify a null response.
    fn is_null(&self) -> bool {
        self.iter().all(|byte| *byte == 0)
    }
}

mod driver {
    use std::fs::File;
    use std::io::Read;
    use std::io::Seek;
    use std::io::Write;
    use std::os::fd::AsFd;

    use crate::Error;
    use crate::Result;

    pub struct Driver {
        /// The file descriptor used to send commands.
        command_fd: File,
        /// The file descriptor used to receive responses.
        response_fd: File,
    }

    impl Driver {
        pub fn new() -> Result<Self> {
            let command_fd = std::fs::OpenOptions::new()
                .write(true)
                .open("/sys/kernel/debug/usb/ucsi/USBC000:00/command")?;

            let mut response_fd = std::fs::OpenOptions::new()
                .read(true)
                .open("/sys/kernel/debug/usb/ucsi/USBC000:00/response")?;

            response_fd.seek(std::io::SeekFrom::Start(0))?;

            Ok(Self {
                command_fd,
                response_fd,
            })
        }

        pub fn submit_command(&mut self, command: &[u8]) -> Result<usize> {
            Ok(self.command_fd.write(command)?)
        }

        pub fn wait_response(&mut self) -> Result<Vec<u8>> {
            const TIMEOUT_10SEC: u16 = 10000;
            let poll_fd =
                nix::poll::PollFd::new(self.response_fd.as_fd(), nix::poll::PollFlags::POLLIN);
            let timeout = nix::poll::PollTimeout::from(TIMEOUT_10SEC);

            match nix::poll::poll(&mut [poll_fd], timeout) {
                Ok(0) => {
                    // Timeout
                    Err(Error::TimeoutError {
                        #[cfg(feature = "backtrace")]
                        backtrace: std::backtrace::Backtrace::capture(),
                    })
                }
                Ok(_) => {
                    let mut response = Vec::new();
                    self.response_fd.read_to_end(&mut response)?;
                    self.response_fd.seek(std::io::SeekFrom::Start(0))?;
                    Ok(response)
                }
                Err(errno) => Err(errno.into()),
            }
        }
    }
}

use driver::Driver;

pub struct UcsiDebugfsBackend {
    /// The driver abstraction.
    driver: Driver,
}

impl UcsiDebugfsBackend {
    /// Instantiates a new UCSI backend for Linux.
    pub fn new() -> Result<Self> {
        let driver = Driver::new()?;
        Ok(Self { driver })
    }

    /// Parses the response from the Linux UCSI driver. It currently replies
    /// with two u64s because it conforms to UCSI 1.2.
    fn parse_response(response: Vec<u8>) -> Result<Vec<u8>> {
        let response = std::str::from_utf8(&response)?;
        // Remove the "0x" prefix and \n at the end
        let hex_string = &response[2..response.len() - 1];
        let (first, second) = hex_string.split_at(16);

        let high = u64::from_str_radix(first, 16).unwrap();
        let low = u64::from_str_radix(second, 16).unwrap();

        let mut result = Vec::new();
        result.extend(&low.to_ne_bytes());
        result.extend(&high.to_ne_bytes());

        Ok(result)
    }

    /// Builds a u64 value from a UCSI command.
    fn build_command_value(command: &Command) -> Result<u64> {
        let mut buf = [0; 8];
        let mut bw = crate::BitWriter::new(Cursor::new(&mut buf[..]));
        let mut val = 0u64;

        command.to_bytes(&mut bw)?;
        for byte in buf.iter().rev() {
            val = (val << 8) | u64::from(*byte);
        }

        Ok(val)
    }

    /// Converts a u64 value to a C string.
    fn stringify_command_val(val: u64) -> Result<Vec<u8>> {
        let c_string = std::ffi::CString::new(val.to_string())?;
        Ok(c_string.into_bytes_with_nul())
    }

    /// Execute the command, returning a string of bytes as a result.
    pub fn execute(&mut self, command: Command) -> Result<Vec<u8>> {
        let cmd_val = Self::build_command_value(&command)?;
        let cmd_str = Self::stringify_command_val(cmd_val)?;

        self.driver.submit_command(&cmd_str)?;

        let response = self.driver.wait_response()?;
        Self::parse_response(response)
    }
}

impl OsBackend for UcsiDebugfsBackend {
    fn capabilities(&mut self) -> Result<Capability> {
        let cmd = Command::GetCapability;
        let response = self.execute(cmd)?;
        let mut bitreader = BitReader::new(Cursor::new(&response[..]));
        Capability::from_bytes(&mut bitreader)
    }

    fn connector_capabilties(&mut self, connector_nr: usize) -> Result<ConnectorCapability> {
        let cmd = Command::GetConnectorCapability { connector_nr };
        let response = self.execute(cmd)?;
        let mut bitreader = BitReader::new(Cursor::new(&response[..]));
        ConnectorCapability::from_bytes(&mut bitreader)
    }

    fn alternate_modes(
        &mut self,
        recipient: GetAlternateModesRecipient,
        connector_nr: usize,
    ) -> Result<Vec<AlternateMode>> {
        let mut alternate_modes = vec![];
        let mut offset = 0;
        loop {
            let cmd = Command::GetAlternateModes {
                recipient,
                connector_nr,
                offset,
            };

            let response = self.execute(cmd)?;
            if response.is_null() {
                break;
            }

            let mut bitreader = BitReader::new(Cursor::new(&response[..]));
            alternate_modes.push(AlternateMode::from_bytes(&mut bitreader)?);
            offset += 1;
        }

        Ok(alternate_modes)
    }

    fn cable_properties(&mut self, connector_nr: usize) -> Result<CableProperty> {
        let cmd = Command::GetCableProperty { connector_nr };
        let response = self.execute(cmd)?;
        let mut bitreader = BitReader::new(Cursor::new(&response[..]));
        CableProperty::from_bytes(&mut bitreader)
    }

    fn connector_status(&mut self, _: usize) -> Result<ConnectorStatus> {
        Err(Error::NotSupported {
            #[cfg(feature = "backtrace")]
            backtrace: std::backtrace::Backtrace::capture(),
        })
    }

    fn pd_message(
        &mut self,
        _: usize,
        _: MessageRecipient,
        _: MessageResponseType,
    ) -> Result<Message> {
        Err(Error::NotSupported {
            #[cfg(feature = "backtrace")]
            backtrace: std::backtrace::Backtrace::capture(),
        })
    }

    fn pdos(
        &mut self,
        connector_nr: usize,
        partner_pdo: bool,
        pdo_offset: u32,
        nr_pdos: usize,
        pdo_type: PdoType,
        source_capabilities_type: PdoSourceCapabilitiesType,
        revision: BcdWrapper,
    ) -> Result<Vec<crate::pd::Pdo>> {
        let mut pdos = vec![];
        let mut nr_pdos_returned = 0;
        loop {
            if nr_pdos > 0 && nr_pdos_returned == nr_pdos {
                break;
            }

            let cmd = Command::GetPdos {
                connector_nr,
                partner_pdo,
                pdo_offset,
                nr_pdos,
                pdo_type,
                source_capabilities_type,
            };

            let response = self.execute(cmd)?;
            if response.is_null() {
                break;
            }

            let mut bitreader = BitReader::new(Cursor::new(&response[..]));
            let pdo = Pdo::from_bytes(&mut bitreader, revision)?;
            pdos.push(pdo);

            nr_pdos_returned += 1;
        }

        Ok(pdos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl From<Driver> for UcsiDebugfsBackend {
        fn from(mock: Driver) -> Self {
            Self { driver: mock }
        }
    }

    #[test]
    fn test_stringify_command_val() {
        let val = 12345u64;
        let result = UcsiDebugfsBackend::stringify_command_val(val).unwrap();
        let expected = format!("{}\0", val).into_bytes();

        assert_eq!(result, expected);
    }

    #[test]
    fn test_build_command_value_get_connector_capability() {
        let command = Command::GetConnectorCapability { connector_nr: 0 };
        let result = UcsiDebugfsBackend::build_command_value(&command).unwrap();
        let expected = 0x10007;

        assert_eq!(result, expected);
    }
}
