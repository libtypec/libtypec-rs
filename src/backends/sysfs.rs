// SPDX-License-Identifier: Apache-2.0 OR MIT
// SPDX-FileCopyrightText: © 2024 Google
// Ported from libtypec (Rajaram Regupathy <rajaram.regupathy@gmail.com>)

//! The sysfs backend

use regex::Regex;
use walkdir::WalkDir;

use std::path::Path;
use std::path::PathBuf;

use crate::pd::Message;
use crate::pd::MessageRecipient;
use crate::pd::MessageResponseType;
use crate::pd::Pdo;
use crate::ucsi::AlternateMode;
use crate::ucsi::CableProperty;
use crate::ucsi::Capability;
use crate::ucsi::ConnectorCapability;
use crate::ucsi::ConnectorCapabilityOperationMode;
use crate::ucsi::ConnectorStatus;
use crate::ucsi::GetAlternateModesRecipient;
use crate::ucsi::PdoSourceCapabilitiesType;
use crate::ucsi::PdoType;
use crate::BcdWrapper;
use crate::Error;
use crate::OsBackend;
use crate::Result;

use sysfs_reader::SysfsReader;

const SYSFS_TYPEC_PATH: &str = "/sys/class/typec";
const SYSFS_PSY_PATH: &str = "/sys/class/power_supply";

/// Creates a `PathBuf` from a string and returns an error if the path does not
/// exist.
fn check_path(path: &str) -> Result<PathBuf> {
    let path = PathBuf::from(path);
    if !path.exists() {
        Err(Error::NotSupported {
            #[cfg(feature = "backtrace")]
            backtrace: std::backtrace::Backtrace::capture(),
        })
    } else {
        Ok(path)
    }
}

pub mod sysfs_reader {
    use std::io;
    use std::io::Cursor;
    use std::path::Path;
    use std::path::PathBuf;

    use crate::pd::pd3p2::vdo::CertStat;
    use crate::pd::pd3p2::vdo::IdHeader;
    use crate::pd::pd3p2::vdo::Product;
    use crate::pd::pd3p2::vdo::ProductType;
    use crate::pd::pd3p2::BatterySupplyPdo;
    use crate::pd::pd3p2::DiscoverIdentityResponse;
    use crate::pd::pd3p2::FastRoleSwap;
    use crate::pd::pd3p2::FixedSupplyPdo;
    use crate::pd::pd3p2::SprProgrammableSupplyPdo;
    use crate::pd::pd3p2::VariableSupplyPdo;
    use crate::pd::MessageRecipient;
    use crate::ucsi::CablePropertyPlugEndType;
    use crate::ucsi::CablePropertyType;
    use crate::ucsi::ConnectorCapabilityOperationMode;
    use crate::ucsi::PdoType;
    use crate::BcdWrapper;
    use crate::BitReader;
    use crate::Error;
    use crate::FromBytes;
    use crate::Result;

    use super::SYSFS_TYPEC_PATH;

    pub struct SysfsReader(Option<PathBuf>);

    impl SysfsReader {
        pub fn new() -> Result<Self> {
            Ok(Self(None))
        }

        pub fn set_path(&mut self, path: &str) -> Result<()> {
            self.0 = Some(super::check_path(path)?);
            Ok(())
        }

        fn read_file(&mut self) -> Result<String> {
            let path = self.0.take().expect("Path not set");
            let string = std::fs::read_to_string(path)?;
            Ok(string)
        }

        pub fn read_bcd(&mut self) -> Result<BcdWrapper> {
            let content = self.read_file()?;
            let mut chars = content.chars();

            let high = chars
                .next()
                .ok_or(io::Error::new(io::ErrorKind::InvalidData, "File is empty"))?;
            let _ = chars.next().ok_or(io::Error::new(
                io::ErrorKind::InvalidData,
                "File is too short",
            ))?;

            // Sometimes we get simply "2"
            let low = chars.next().unwrap_or('0');

            let high = high.to_digit(10).ok_or(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid digit: {high}"),
            ))?;
            let low = low.to_digit(10).ok_or(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid digit: {low}"),
            ))?;

            let bcd = (high << 8) | low;

            Ok(BcdWrapper(bcd))
        }

        pub fn read_opr(&mut self) -> Result<ConnectorCapabilityOperationMode> {
            let content = self.read_file()?;
            if content.contains("source") {
                if content.contains("sink") {
                    Ok(ConnectorCapabilityOperationMode::DRP)
                } else {
                    Ok(ConnectorCapabilityOperationMode::RP_ONLY)
                }
            } else {
                Ok(ConnectorCapabilityOperationMode::RD_ONLY)
            }
        }

        pub fn read_pd_revision(&mut self) -> Result<u8> {
            let content = self.read_file()?;
            let mut chars = content.chars();

            let b0 = chars.next().ok_or(io::Error::new(
                io::ErrorKind::InvalidData,
                "File is too short",
            ))?;
            let _ = chars.next().ok_or(io::Error::new(
                io::ErrorKind::InvalidData,
                "File is too short",
            ))?;
            let b2 = chars.next().ok_or(io::Error::new(
                io::ErrorKind::InvalidData,
                "File is too short",
            ))?;

            let b0_digit = b0.to_digit(10).ok_or(io::Error::new(
                io::ErrorKind::InvalidData,
                "Non-digit character found",
            ))? as u8;

            let b2_digit = b2.to_digit(10).ok_or(io::Error::new(
                io::ErrorKind::InvalidData,
                "Non-digit character found",
            ))? as u8;

            let rev = (b0_digit << 4) | b2_digit;

            Ok(rev)
        }

        pub fn read_hex_u32(&mut self) -> Result<u32> {
            let content = self.read_file()?.replace("0x", "");
            let hex = u32::from_str_radix(content.trim(), 16).map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidData, "Could not parse hex value")
            })?;
            Ok(hex)
        }

        pub fn read_u32(&mut self) -> Result<u32> {
            let mut content = self.read_file()?;
            content.retain(|c| c.is_ascii_digit());

            let dword = content.trim().parse::<u32>().map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidData, "Could not parse u32 value")
            })?;
            Ok(dword)
        }

        pub fn read_bit(&mut self) -> Result<bool> {
            let content = self.read_file()?;
            let bit = content.trim().parse::<bool>().map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidData, "Could not parse bool value")
            })?;
            Ok(bit)
        }

        pub fn read_cable_plug_type(&mut self) -> Result<CablePropertyPlugEndType> {
            let content = self.read_file()?;
            let plug_type = if content.contains("type-c") {
                CablePropertyPlugEndType::UsbTypeC
            } else if content.contains("type-a") {
                CablePropertyPlugEndType::UsbTypeA
            } else if content.contains("type-b") {
                CablePropertyPlugEndType::UsbTypeB
            } else {
                CablePropertyPlugEndType::OtherNotUsb
            };

            Ok(plug_type)
        }

        pub fn read_cable_type(&mut self) -> Result<CablePropertyType> {
            let content = self.read_file()?;
            let cable_type = if content.contains("active") {
                CablePropertyType::Active
            } else if content.contains("passive") {
                CablePropertyType::Passive
            } else {
                return Err(Error::ParseStringError {
                    field: "cable_type".to_string(),
                    value: content,
                    #[cfg(feature = "backtrace")]
                    backtrace: std::backtrace::Backtrace::capture(),
                });
            };

            Ok(cable_type)
        }

        pub fn read_cable_mode_support(&mut self) -> Result<bool> {
            let content = self.read_file()?;
            let mode_support = match content.chars().next() {
                Some('0') => false,
                Some(_) => true,
                None => {
                    return Err(Error::ParseStringError {
                        field: "cable_mode_support".to_string(),
                        value: content,
                        #[cfg(feature = "backtrace")]
                        backtrace: std::backtrace::Backtrace::capture(),
                    });
                }
            };

            Ok(mode_support)
        }

        pub fn read_fixed_supply_pdo(
            &mut self,
            path: &Path,
            src_or_sink: PdoType,
        ) -> Result<FixedSupplyPdo> {
            match src_or_sink {
                PdoType::Source => {
                    self.set_path(&path.join("dual_role_power").to_string_lossy())?;
                    let dual_role_power = self.read_bit()?;
                    self.set_path(&path.join("higher_capability").to_string_lossy())?;
                    let higher_capability = self.read_bit()?;
                    self.set_path(&path.join("unconstrained_power").to_string_lossy())?;
                    let unconstrained_power = self.read_bit()?;
                    self.set_path(&path.join("usb_communication_capable").to_string_lossy())?;
                    let usb_communications_capable = self.read_bit()?;
                    self.set_path(&path.join("dual_role_data").to_string_lossy())?;
                    let dual_role_data = self.read_bit()?;
                    self.set_path(&path.join("fast_role_swap").to_string_lossy())?;
                    let fast_role_swap = self.read_u32()?;
                    let fast_role_swap =
                        FastRoleSwap::n(fast_role_swap).ok_or_else(|| Error::ParseError {
                            field: "fast_role_swap".into(),
                            value: fast_role_swap,
                            #[cfg(feature = "backtrace")]
                            backtrace: std::backtrace::Backtrace::capture(),
                        })?;
                    self.set_path(&path.join("voltage").to_string_lossy())?;
                    let voltage = (self.read_u32()? / 50).into();
                    self.set_path(&path.join("maximum_current").to_string_lossy())?;
                    let operational_current = (self.read_u32()? / 10).into();

                    Ok(FixedSupplyPdo {
                        dual_role_power,
                        higher_capability,
                        unconstrained_power,
                        usb_communications_capable,
                        dual_role_data,
                        fast_role_swap,
                        voltage,
                        operational_current,
                    })
                }
                PdoType::Sink => {
                    self.set_path(&path.join("dual_role_power").to_string_lossy())?;
                    let dual_role_power = self.read_bit()?;
                    self.set_path(&path.join("higher_capability").to_string_lossy())?;
                    let higher_capability = self.read_bit()?;
                    self.set_path(&path.join("unconstrained_power").to_string_lossy())?;
                    let unconstrained_power = self.read_bit()?;
                    self.set_path(&path.join("usb_communication_capable").to_string_lossy())?;
                    let usb_communications_capable = self.read_bit()?;
                    self.set_path(&path.join("dual_role_data").to_string_lossy())?;
                    let dual_role_data = self.read_bit()?;
                    self.set_path(&path.join("fast_role_swap_current").to_string_lossy())?;
                    let fast_role_swap = self.read_u32()?;
                    let fast_role_swap =
                        FastRoleSwap::n(fast_role_swap).ok_or_else(|| Error::ParseError {
                            field: "fast_role_swap".into(),
                            value: fast_role_swap,
                            #[cfg(feature = "backtrace")]
                            backtrace: std::backtrace::Backtrace::capture(),
                        })?;
                    self.set_path(&path.join("voltage").to_string_lossy())?;
                    let voltage = (self.read_u32()? / 50).into();
                    self.set_path(&path.join("operational_current").to_string_lossy())?;
                    let operational_current = (self.read_u32()? / 10).into();

                    Ok(FixedSupplyPdo {
                        dual_role_power,
                        higher_capability,
                        unconstrained_power,
                        usb_communications_capable,
                        dual_role_data,
                        fast_role_swap,
                        voltage,
                        operational_current,
                    })
                }
            }
        }

        pub fn read_programmable_supply_pdo(
            &mut self,
            path: &Path,
            src_or_sink: PdoType,
        ) -> Result<SprProgrammableSupplyPdo> {
            self.set_path(&path.join("maximum_voltage").to_string_lossy())?;
            let max_voltage = (self.read_u32()? / 50).into();
            self.set_path(&path.join("minimum_voltage").to_string_lossy())?;
            let min_voltage = (self.read_u32()? / 50).into();
            let max_current = (match src_or_sink {
                PdoType::Source => {
                    self.set_path(&path.join("maximum_current").to_string_lossy())?;
                    self.read_u32()?
                }
                PdoType::Sink => {
                    self.set_path(&path.join("operational_current").to_string_lossy())?;
                    self.read_u32()?
                }
            } / 10)
                .into();

            Ok(SprProgrammableSupplyPdo {
                max_voltage,
                min_voltage,
                max_current,
            })
        }

        pub fn read_battery_supply_pdo(
            &mut self,
            path: &Path,
            src_or_sink: PdoType,
        ) -> Result<BatterySupplyPdo> {
            self.set_path(&path.join("maximum_voltage").to_string_lossy())?;
            let max_voltage = (self.read_u32()? / 50).into();
            self.set_path(&path.join("minimum_voltage").to_string_lossy())?;
            let min_voltage = (self.read_u32()? / 50).into();
            let operational_power = (match src_or_sink {
                PdoType::Source => {
                    self.set_path(&path.join("maximum_power").to_string_lossy())?;
                    self.read_u32()?
                }
                PdoType::Sink => {
                    self.set_path(&path.join("operational_power").to_string_lossy())?;
                    self.read_u32()?
                }
            } / 250)
                .into();

            Ok(BatterySupplyPdo {
                max_voltage,
                min_voltage,
                operational_power,
            })
        }

        pub fn read_variable_supply_pdo(
            &mut self,
            path: &Path,
            _src_or_sink: PdoType,
        ) -> Result<VariableSupplyPdo> {
            self.set_path(&path.join("maximum_voltage").to_string_lossy())?;
            let max_voltage = (self.read_u32()? / 100).into();
            self.set_path(&path.join("minimum_voltage").to_string_lossy())?;
            let min_voltage = (self.read_u32()? / 100).into();
            self.set_path(&path.join("maximum_current").to_string_lossy())?;
            let max_current = (self.read_u32()? / 50).into();

            Ok(VariableSupplyPdo {
                max_voltage,
                min_voltage,
                max_current,
            })
        }

        pub fn discover_identity(
            &mut self,
            conn_num: usize,
            recipient: MessageRecipient,
        ) -> Result<DiscoverIdentityResponse> {
            let (cert_stat, id_header, product, product_type_vdo) = match recipient {
                MessageRecipient::Sop => {
                    let path_str =
                        format!("{}/port{}-partner/identity", SYSFS_TYPEC_PATH, conn_num);
                    self.read_identity(&path_str)?
                }
                MessageRecipient::SopPrime => {
                    let path_str = format!("{}/port{}-cable/identity", SYSFS_TYPEC_PATH, conn_num);
                    self.read_identity(&path_str)?
                }
                _ => {
                    return Err(Error::NotSupported {
                        #[cfg(feature = "backtrace")]
                        backtrace: std::backtrace::Backtrace::capture(),
                    })
                }
            };

            let binding = id_header.to_le_bytes();
            let mut br = BitReader::new(Cursor::new(&binding));
            let id_header_vdo = IdHeader::from_bytes(&mut br)?;

            let binding = cert_stat.to_le_bytes();
            let mut br = BitReader::new(Cursor::new(&binding));
            let cert_stat = CertStat::from_bytes(&mut br)?;

            let binding = product.to_le_bytes();
            let mut br = BitReader::new(Cursor::new(&binding));
            let product_vdo = Product::from_bytes(&mut br)?;

            Ok(DiscoverIdentityResponse {
                header: Default::default(),
                id_header_vdo,
                cert_stat,
                product_vdo,
                product_type_vdo,
            })
        }

        fn read_identity(&mut self, path: &str) -> Result<(u32, u32, u32, [ProductType; 3])> {
            self.set_path(&format!("{}/{}", path, "cert_stat"))?;
            let cert_stat = self.read_u32()?;
            self.set_path(&format!("{}/{}", path, "id_header"))?;
            let id_header = self.read_u32()?;
            self.set_path(&format!("{}/{}", path, "product"))?;
            let product = self.read_u32()?;
            let mut product_type_vdo = [
                ProductType::default(),
                ProductType::default(),
                ProductType::default(),
            ];
            for (i, vdo) in product_type_vdo.iter_mut().enumerate() {
                self.set_path(&format!("{}/product_type_vdo{}", path, i + 1))?;
                let value = self.read_u32()?;
                if value != 0 {
                    *vdo = ProductType::n(value).ok_or(Error::ParseError {
                        field: "product_type_vdo".to_string(),
                        value,
                        #[cfg(feature = "backtrace")]
                        backtrace: std::backtrace::Backtrace::capture(),
                    })?;
                }
            }
            Ok((cert_stat, id_header, product, product_type_vdo))
        }
    }
}

pub struct SysfsBackend {
    /// Reads the sysfs files.
    reader: SysfsReader,
}

impl SysfsBackend {
    /// Initializes the sysfs backend.
    pub fn new() -> Result<Self> {
        if WalkDir::new(SYSFS_TYPEC_PATH).into_iter().count() == 1 {
            return Err(Error::NotSupported {
                #[cfg(feature = "backtrace")]
                backtrace: std::backtrace::Backtrace::capture(),
            });
        }

        Ok(Self {
            reader: SysfsReader::new()?,
        })
    }
}

impl OsBackend for SysfsBackend {
    fn capabilities(&mut self) -> Result<Capability> {
        let mut num_ports = 0;
        let mut num_alt_modes = 0;
        let mut pd_version = Default::default();
        let mut usb_type_c_version = Default::default();

        for entry in WalkDir::new(SYSFS_TYPEC_PATH) {
            let entry = entry?;
            let entry_name = entry.file_name().to_string_lossy();

            let re = Regex::new(r"^port\d+$").unwrap();
            if re.is_match(&entry_name) {
                num_ports += 1;
                for port_entry in WalkDir::new(entry.path()) {
                    let port_entry = port_entry?;
                    let port_entry_name = port_entry.file_name().to_string_lossy();

                    let re = Regex::new(r"^port\d\.\d$").unwrap();
                    if re.is_match(&port_entry_name) {
                        num_alt_modes += 1;
                    }
                }

                let port_content_path =
                    format!("{}/usb_power_delivery_revision", entry.path().display());
                self.reader.set_path(&port_content_path)?;
                pd_version = self.reader.read_bcd()?;

                let port_content_path = format!("{}/usb_typec_revision", entry.path().display());
                self.reader.set_path(&port_content_path)?;
                usb_type_c_version = self.reader.read_bcd()?;
            }
        }

        let capabilities = Capability {
            num_connectors: num_ports,
            num_alt_modes,
            pd_version,
            usb_type_c_version,
            ..Default::default()
        };

        Ok(capabilities)
    }

    fn connector_capabilties(
        &mut self,
        connector_nr: usize,
    ) -> Result<crate::ucsi::ConnectorCapability> {
        let path_str = format!("{SYSFS_TYPEC_PATH}/port{}", connector_nr);

        let port_content = format!("{}/{}", path_str, "power_role");
        self.reader.set_path(&port_content)?;

        let mut connector_capabilities = ConnectorCapability {
            operation_mode: self.reader.read_opr()?,
            ..Default::default()
        };

        match connector_capabilities.operation_mode {
            ConnectorCapabilityOperationMode::DRP => {
                connector_capabilities.provider = true;
                connector_capabilities.consumer = true;
            }
            ConnectorCapabilityOperationMode::RD_ONLY => {
                connector_capabilities.consumer = true;
            }
            _ => {
                connector_capabilities.provider = true;
            }
        }

        if crate::is_chrome_os() {
            let port_content = format!(
                "{}/port{}-partner/{}",
                path_str, connector_nr, "usb_power_delivery_revision"
            );

            self.reader.set_path(&port_content)?;
            connector_capabilities.partner_pd_revision = self.reader.read_pd_revision()?;
        }

        Ok(connector_capabilities)
    }

    fn alternate_modes(
        &mut self,
        recipient: GetAlternateModesRecipient,
        connector_nr: usize,
    ) -> Result<Vec<AlternateMode>> {
        let mut alt_modes = vec![];

        loop {
            let num_alt_mode = alt_modes.len();
            let path_str = match recipient {
                crate::ucsi::GetAlternateModesRecipient::Connector => {
                    format!(
                        "{}/port{}/port{}.{}",
                        SYSFS_TYPEC_PATH, connector_nr, connector_nr, num_alt_mode
                    )
                }
                crate::ucsi::GetAlternateModesRecipient::Sop => {
                    format!(
                        "{}/port{}/port{}-partner/port{}-partner.{}",
                        SYSFS_TYPEC_PATH, connector_nr, connector_nr, connector_nr, num_alt_mode
                    )
                }
                crate::ucsi::GetAlternateModesRecipient::SopPrime => {
                    format!(
                        "{}/port{}-cable/port{}-plug0/port{}-plug0.{}",
                        SYSFS_TYPEC_PATH, connector_nr, connector_nr, connector_nr, num_alt_mode
                    )
                }
                _ => {
                    return Err(Error::NotSupported {
                        #[cfg(feature = "backtrace")]
                        backtrace: std::backtrace::Backtrace::capture(),
                    })
                }
            };

            let mut alt_mode = crate::ucsi::AlternateMode::default();

            let svid_path = format!("{}/{}", path_str, "svid");
            if self.reader.set_path(&svid_path).is_err() {
                break;
            }

            alt_mode.svid[0] = self.reader.read_hex_u32()?;

            let vdo_path = format!("{}/{}", path_str, "vdo");
            if self.reader.set_path(&vdo_path).is_err() {
                break;
            }

            alt_mode.vdo[0] = self.reader.read_hex_u32()?;
            alt_modes.push(alt_mode);
        }

        Ok(alt_modes)
    }

    fn cable_properties(&mut self, connector_nr: usize) -> Result<CableProperty> {
        let mut cable_property = CableProperty::default();
        let path_str = format!("{}/port{}-cable", SYSFS_TYPEC_PATH, connector_nr);

        let plug_type_path = format!("{}/{}", path_str, "plug_type");
        self.reader.set_path(&plug_type_path)?;
        cable_property.plug_end_type = self.reader.read_cable_plug_type()?;

        let cable_type_path = format!("{}/{}", path_str, "type");
        self.reader.set_path(&cable_type_path)?;
        cable_property.cable_type = self.reader.read_cable_type()?;

        let mode_support_path = format!(
            "{}/port{}-plug0/{}",
            SYSFS_TYPEC_PATH, connector_nr, "number_of_alternate_modes"
        );
        self.reader.set_path(&mode_support_path)?;
        cable_property.mode_support = self.reader.read_cable_mode_support()?;

        Ok(cable_property)
    }

    fn connector_status(&mut self, connector_nr: usize) -> Result<ConnectorStatus> {
        let mut connector_status = ConnectorStatus::default();

        let partner_path_str = format!(
            "{}/port{}/port{}-partner",
            SYSFS_TYPEC_PATH, connector_nr, connector_nr
        );
        connector_status.connect_status = Path::new(&partner_path_str).exists();

        let psy_path_str = format!(
            "{}/ucsi-source-psy-USBC000:00{}",
            SYSFS_PSY_PATH,
            connector_nr + 1
        );

        let online_path = format!("{}/{}", psy_path_str, "online");
        self.reader.set_path(&online_path)?;
        let ret = self.reader.read_hex_u32()?;

        if ret != 0 {
            let current_now_path = format!("{}/{}", psy_path_str, "current_now");
            self.reader.set_path(&current_now_path)?;
            let cur = self.reader.read_u32()? / 1000;

            let voltage_now_path = format!("{}/{}", psy_path_str, "voltage_now");
            self.reader.set_path(&voltage_now_path)?;
            let volt = self.reader.read_u32()? / 1000;

            let op_mw = (cur * volt) / (250 * 1000);

            let current_max_path = format!("{}/{}", psy_path_str, "current_max");
            self.reader.set_path(&current_max_path)?;
            let cur = self.reader.read_u32()? / 1000;

            let voltage_max_path = format!("{}/{}", psy_path_str, "voltage_max");
            self.reader.set_path(&voltage_max_path)?;
            let volt = self.reader.read_u32()? / 1000;

            let max_mw = (cur * volt) / (250 * 1000);

            connector_status.negotiated_power_level = (op_mw << 10) | (max_mw) & 0x3ff;
        }

        Ok(connector_status)
    }

    fn pd_message(
        &mut self,
        connector_nr: usize,
        recipient: MessageRecipient,
        response_type: MessageResponseType,
    ) -> Result<Message> {
        match response_type {
            MessageResponseType::DiscoverIdentity => Ok(Message::Pd3p2DiscoverIdentityResponse(
                self.reader.discover_identity(connector_nr, recipient)?,
            )),
            _ => Err(Error::NotSupported {
                #[cfg(feature = "backtrace")]
                backtrace: std::backtrace::Backtrace::capture(),
            }),
        }
    }

    fn pdos(
        &mut self,
        connector_nr: usize,
        partner_pdo: bool,
        _pdo_offset: u32,
        _nr_pdos: usize,
        pdo_type: PdoType,
        _source_capabilities_type: PdoSourceCapabilitiesType,
        _revision: BcdWrapper,
    ) -> Result<Vec<crate::pd::Pdo>> {
        let mut pdos = Vec::new();

        let path_str = if partner_pdo {
            match pdo_type {
                PdoType::Source => {
                    format!(
                        "{}/port{}-partner/usb_power_delivery/source-capabilities",
                        SYSFS_TYPEC_PATH, connector_nr
                    )
                }
                PdoType::Sink => {
                    format!(
                        "{}/port{}-partner/usb_power_delivery/sink-capabilities",
                        SYSFS_TYPEC_PATH, connector_nr
                    )
                }
            }
        } else {
            match pdo_type {
                PdoType::Source => {
                    format!(
                        "{}/port{}/usb_power_delivery/source-capabilities",
                        SYSFS_TYPEC_PATH, connector_nr
                    )
                }
                PdoType::Sink => {
                    format!(
                        "{}/port{}/usb_power_delivery/sink-capabilities",
                        SYSFS_TYPEC_PATH, connector_nr
                    )
                }
            }
        };

        let port_path = format!("{SYSFS_TYPEC_PATH}/port{connector_nr}");
        for entry in WalkDir::new(port_path) {
            let entry = entry?;
            let entry_name = entry.file_name().to_string_lossy();
            let port_path = format!("{path_str}/{entry_name}");
            let port_path = Path::new(&port_path);

            let pdo = if entry_name.contains("fixed") {
                Pdo::Pd3p2FixedSupplyPdo(self.reader.read_fixed_supply_pdo(port_path, pdo_type)?)
            } else if entry_name.contains("variable") {
                Pdo::Pd3p2VariableSupplyPdo(
                    self.reader.read_variable_supply_pdo(port_path, pdo_type)?,
                )
            } else if entry_name.contains("battery") {
                Pdo::Pd3p2BatterySupplyPdo(
                    self.reader.read_battery_supply_pdo(port_path, pdo_type)?,
                )
            } else if entry_name.contains("programmable") {
                Pdo::Pd3p2AugmentedPdo(
                    self.reader
                        .read_programmable_supply_pdo(port_path, pdo_type)?,
                )
            } else {
                continue;
            };

            pdos.push(pdo);
        }

        Ok(pdos)
    }
}
