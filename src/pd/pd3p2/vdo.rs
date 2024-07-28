// SPDX-License-Identifier: Apache-2.0 OR MIT
// SPDX-FileCopyrightText: © 2024 Google
// Ported from libtypec (Rajaram Regupathy <rajaram.regupathy@gmail.com>)

//! USB Power Delivery 3.2 (PD3.2) Vendor Defined Objects.

#[cfg(feature = "backtrace")]
use std::backtrace::Backtrace;
use std::ffi::CString;

use bitstream_io::BitRead;
use enumn::N;
use proc_macros::CApiWrapper;

use crate::BcdWrapper;
use crate::BitReader;
use crate::Error;
use crate::FromBytes;
use crate::Milliohm;
use crate::Result;

/// Maximum VPD VBUS Voltage
#[derive(Debug, Clone, PartialEq, Default, N, Copy, CApiWrapper)]
#[c_api(prefix = "Pd3p2Vdo", repr_c)]
pub enum MaxVbusVoltage {
    #[default]
    /// 20V
    V20 = 0,
    /// 30V (Deprecated)
    V30,
    /// 40V (Deprecated)
    V40,
    /// 50V (Deprecated)
    V50,
}

/// Charge Through Support
#[derive(Debug, Clone, PartialEq, Default, N, Copy, CApiWrapper)]
#[c_api(prefix = "Pd3p2Vdo", repr_c)]
pub enum ChargeThroughSupport {
    #[default]
    /// the VPD does not support Charge Through
    NotSupported = 0,
    /// the VPD supports Charge Through
    Supported,
}

/// VPD VDO. USB PD 3.2 VPD VDO (Section 6.4.4.3.1.9)
#[derive(Debug, Clone, PartialEq, Default, CApiWrapper)]
#[c_api(prefix = "Pd3p2Vdo", repr_c)]
pub struct Vpd {
    /// HW Version 0000b…1111b assigned by the VID owner
    pub hw_version: u8,
    /// Firmware Version 0000b…1111b assigned by the VID owner
    pub firmware_version: u8,
    /// Version Number of the VDO (not this specification Version)
    pub vdo_version: u8,
    /// Maximum VPD VBUS Voltage
    pub max_vbus_voltage: MaxVbusVoltage,
    /// Charge Through Current Support
    pub charge_through_current_support: bool,
    /// VBUS Impedance
    pub vbus_impedance: Milliohm,
    /// Ground Impedance
    pub ground_impedance: Milliohm,
    /// Charge Through Support
    pub charge_through_support: ChargeThroughSupport,
}

impl FromBytes for Vpd {
    fn from_bytes(bit_reader: &mut BitReader) -> Result<Self> {
        let hw_version = bit_reader.read(4)?;
        let firmware_version = bit_reader.read(4)?;
        let vdo_version = bit_reader.read(3)?;
        let max_vbus_voltage = bit_reader.read(2)?;
        let max_vbus_voltage =
            MaxVbusVoltage::n(max_vbus_voltage).ok_or_else(|| Error::ParseError {
                field: "max_vbus_voltage".into(),
                value: max_vbus_voltage,
                #[cfg(feature = "backtrace")]
                backtrace: Backtrace::capture(),
            })?;
        let charge_through_current_support = bit_reader.read_bit()?;
        let vbus_impedance = bit_reader.read::<u32>(6)?.into();
        let ground_impedance = bit_reader.read::<u32>(6)?.into();
        let charge_through_support = bit_reader.read_bit()?;
        let charge_through_support =
            ChargeThroughSupport::n(charge_through_support).ok_or_else(|| Error::ParseError {
                field: "charge_through_support".into(),
                value: u32::from(charge_through_support),
                #[cfg(feature = "backtrace")]
                backtrace: std::backtrace::Backtrace::capture(),
            })?;

        Ok(Self {
            hw_version,
            firmware_version,
            vdo_version,
            max_vbus_voltage,
            charge_through_current_support,
            vbus_impedance,
            ground_impedance,
            charge_through_support,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Default, N, Copy, CApiWrapper)]
#[c_api(prefix = "Pd3p2Vdo", repr_c)]
pub enum UfpVersion {
    /// Version 1.3 = 011b
    #[default]
    Unknown = 0,
    V1_3 = 3,
}

#[derive(Debug, Clone, PartialEq, Default, N, Copy, CApiWrapper)]
#[c_api(prefix = "Pd3p2Vdo", repr_c)]
pub enum UfpDeviceCapability {
    #[default]
    /// [USB 2.0] Device Capable
    Usb2_0 = 0,
    /// [USB 2.0] Device Capable (Billboard only)
    Usb2_0Billboard,
    /// [USB 3.2] Device Capable
    Usb3_2,
    /// [USB4] Device Capable
    Usb4,
}

#[derive(Debug, Clone, PartialEq, Default, N, Copy, CApiWrapper)]
#[c_api(prefix = "Pd3p2Vdo", repr_c)]
pub enum UfpVConnPower {
    #[default]
    /// 1W
    W1 = 0,
    /// 1.5W
    W1_5,
    /// 2W
    W2,
    /// 3W
    W3,
    /// 4W
    W4,
    /// 5W
    W5,
    /// 6W
    W6,
}

#[derive(Debug, Clone, PartialEq, Default, N, Copy, CApiWrapper)]
#[c_api(prefix = "Pd3p2Vdo", repr_c)]
pub enum UfpAlternateModes {
    #[default]
    /// Supports [TBT3] Alternate Mode
    Tbt3 = 0,
    /// Supports Alternate Modes that reconfigure the signals on the [USB Type-C 2.3] connector – except for [TBT3].
    Reconfigurable,
    /// Supports Alternate Modes that do not reconfigure the signals on the [USB Type-C 2.3] connector
    NonReconfigurable,
}
#[derive(Debug, Clone, PartialEq, Default, CApiWrapper)]
#[c_api(prefix = "Pd3p2Vdo", repr_c)]
pub struct Ufp {
    /// Version Number of the VDO (not this specification Version)
    pub ufp_vdo_version: UfpVersion,
    /// Device Capability
    pub device_capability: UfpDeviceCapability,
    /// VCONN Power
    pub vconn_power: UfpVConnPower,
    /// Indicates whether the AMA requires VCONN in order to function.
    pub vconn_required: bool,
    /// Indicates whether the AMA requires VBUS in order to function.
    pub vbus_required: bool,
    /// Alternate Modes
    pub alternate_modes: UfpAlternateModes,
}

impl FromBytes for Ufp {
    fn from_bytes(bit_reader: &mut BitReader) -> Result<Self> {
        let ufp_vdo_version = bit_reader.read(3)?;
        let ufp_vdo_version = UfpVersion::n(ufp_vdo_version).ok_or_else(|| Error::ParseError {
            field: "ufp_vdo_version".into(),
            value: ufp_vdo_version,
            #[cfg(feature = "backtrace")]
            backtrace: std::backtrace::Backtrace::capture(),
        })?;
        bit_reader.skip(1)?; // Skip reserved bit
        let device_capability = bit_reader.read(4)?;
        let device_capability =
            UfpDeviceCapability::n(device_capability).ok_or_else(|| Error::ParseError {
                field: "device_capability".into(),
                value: device_capability,
                #[cfg(feature = "backtrace")]
                backtrace: std::backtrace::Backtrace::capture(),
            })?;
        bit_reader.skip(2)?; // Skip Connector Type (Legacy)
        bit_reader.skip(11)?; // Skip reserved bits
        let vconn_power = bit_reader.read(3)?;
        let vconn_power = UfpVConnPower::n(vconn_power).ok_or_else(|| Error::ParseError {
            field: "vconn_power".into(),
            value: vconn_power,
            #[cfg(feature = "backtrace")]
            backtrace: std::backtrace::Backtrace::capture(),
        })?;
        let vconn_required = bit_reader.read_bit()?;
        let vbus_required = bit_reader.read_bit()?;
        let alternate_modes = bit_reader.read(3)?;
        let alternate_modes =
            UfpAlternateModes::n(alternate_modes).ok_or_else(|| Error::ParseError {
                field: "alternate_modes".into(),
                value: alternate_modes,
                #[cfg(feature = "backtrace")]
                backtrace: std::backtrace::Backtrace::capture(),
            })?;

        Ok(Self {
            ufp_vdo_version,
            device_capability,
            vconn_power,
            vconn_required,
            vbus_required,
            alternate_modes,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Default, N, Copy, CApiWrapper)]
#[c_api(prefix = "Pd3p2Vdo", repr_c)]
pub enum DfpVersion {
    #[default]
    /// Version 1.2 = 010b
    Version12 = 0b010,
    // Values 011b…111b are Reserved and Shall Not be used
}

#[derive(Debug, Clone, PartialEq, Default, N, Copy, CApiWrapper)]
#[c_api(prefix = "Pd3p2Vdo", repr_c)]
pub enum DfpHostCapability {
    #[default]
    /// [USB 2] Host Capable
    Usb20 = 0,
    /// [USB 3] Host Capable
    Usb32 = 1,
    /// [USB 4] Host Capable
    Usb4 = 2,
}

#[derive(Debug, Clone, PartialEq, Default, CApiWrapper)]
#[c_api(prefix = "Pd3p2Vdo", repr_c)]
/// See USB PD 3.2 - 6.4.4.3.1.5 DFP VDO
pub struct Dfp {
    /// Version Number of the VDO (not this specification Version)
    pub dfp_vdo_version: DfpVersion,
    /// Host Capability Bit Description
    pub host_capability: DfpHostCapability,
    /// Unique port number to identify a specific port on a multi-port device
    pub port_number: u32,
}

impl FromBytes for Dfp {
    fn from_bytes(bit_reader: &mut BitReader) -> Result<Self> {
        let dfp_vdo_version = bit_reader.read(3)?;
        let dfp_vdo_version = DfpVersion::n(dfp_vdo_version).ok_or_else(|| Error::ParseError {
            field: "dfp_vdo_version".into(),
            value: dfp_vdo_version,
            #[cfg(feature = "backtrace")]
            backtrace: std::backtrace::Backtrace::capture(),
        })?;

        bit_reader.skip(2)?;

        let host_capability = bit_reader.read(3)?;
        let host_capability =
            DfpHostCapability::n(host_capability).ok_or_else(|| Error::ParseError {
                field: "host_capability".into(),
                value: host_capability,
                #[cfg(feature = "backtrace")]
                backtrace: std::backtrace::Backtrace::capture(),
            })?;

        bit_reader.skip(2)?;

        let port_number = bit_reader.read(5)?;

        Ok(Dfp {
            dfp_vdo_version,
            host_capability,
            port_number,
        })
    }
}

/// The Discover Modes Command returns a list of zero to six VDOs, each of which
/// describes a Mode.
///
/// See 6.4.4.2.4 Object Position in USB-PD
pub const MAX_NUM_ALT_MODE: usize = 6;

#[derive(Debug, Clone, PartialEq, Default, CApiWrapper)]
#[c_api(prefix = "Pd3p2Vdo", repr_c)]
pub struct Product {
    /// Product ID (assigned by the manufacturer)
    product_id: u32,
    /// Device release number.
    device: BcdWrapper,
}

impl FromBytes for Product {
    fn from_bytes(reader: &mut BitReader) -> Result<Self> {
        let product_id = reader.read(16)?;
        let device = reader.read(16)?;

        Ok(Product {
            product_id,
            device: BcdWrapper(device),
        })
    }
}

/// Contains the XID assigned by USB-IF to the product before certification in
/// binary format
///
/// See table 6.38 in the USB PD Specification for more information.
#[derive(Debug, Clone, PartialEq, Default, CApiWrapper)]
#[c_api(prefix = "Pd3p2Vdo", repr_c)]
pub struct CertStat {
    /// The XID assigned by USB-IF to the product before certification in binary
    /// format.
    pub xid: u32,
}

impl FromBytes for CertStat {
    fn from_bytes(reader: &mut BitReader) -> Result<Self> {
        let xid = reader.read(32)?;

        Ok(CertStat { xid })
    }
}

#[derive(Debug, Clone, PartialEq, Default, N, Copy, CApiWrapper)]
#[c_api(prefix = "Pd3p2Vdo", repr_c)]
/// See USBPDB 6.4.4.3.1.4
pub enum SopDfpProductType {
    #[default]
    NotADfp,
    PdUsbHub,
    PdUsbHost,
    PowerBrick,
}

#[derive(Debug, Clone, PartialEq, Default, N, Copy, CApiWrapper)]
#[c_api(prefix = "Pd3p2Vdo", repr_c)]
pub enum SopUfpProductType {
    #[default]
    NotAUfp,
    PdUsbHub,
    PdUsbPeripheral,
    Psd,
    NotACablePlugOrVPD,
    PassiveCable,
    ActiveCable,
    VConnPoweredUsbDevice,
}

#[derive(Debug, Clone, PartialEq, Default, N, Copy, CApiWrapper)]
#[c_api(prefix = "Pd3p2Vdo", repr_c)]
pub enum IdHeaderConnectorType {
    #[default]
    ReservedForCompatibility = 0,
    Reserved = 1,
    TypecReceptacle = 2,
    TypecPlug = 3,
}

#[derive(Debug, Clone, PartialEq, Default, CApiWrapper)]
#[c_api(prefix = "Pd3p2Vdo", repr_c, manual_from_impl)]
pub struct IdHeader {
    #[c_api(opaque)]
    pub vendor: String,
    /// USB Communications Capable as USB Host
    pub usb_host_capability: bool,
    /// USB Communications Capable as a USB Device
    pub usb_device_capability: bool,
    /// Indicates the type of Product when in UFP Data Role, whether a VDO will
    /// be returned and if so the type of VDO to be returned.
    pub sop_product_type_ufp: SopUfpProductType,
    /// Indicates whether or not the Product (either a Cable Plug or a device
    /// that can operate in the UFP role) is capable of supporting Modes.
    pub modal_operation_supported: bool,
    /// Indicates the type of Product when in DFP Data Role, whether a VDO will
    /// be returned and if so the type of VDO to be returned.
    pub sop_product_type_dfp: SopDfpProductType,
    /// A value identifying it as either a USB Type-C® receptacle or a USB
    /// Type-C® plug.
    pub connector_type: IdHeaderConnectorType,
    /// Value of the Vendor ID assigned to them by USB-IF.
    pub usb_vendor_id: u32,
}

impl IdHeader {
    #[no_mangle]
    /// Gets a null-terminated vendor string.
    pub(crate) extern "C" fn Pd3p2VdoIdHeader_get_vendor(
        &self,
        vendor: &mut [u8; 32],
    ) -> std::ffi::c_int {
        let c_str = match CString::new(self.vendor.clone()) {
            Ok(c) => c,
            Err(_) => return -nix::libc::EINVAL,
        };

        let c_str = c_str.to_bytes_with_nul();
        let len = std::cmp::min(c_str.len(), vendor.len());
        vendor.copy_from_slice(&c_str[0..len]);
        0
    }
}

#[cfg(feature = "c_api")]
impl From<Pd3p2VdoIdHeader> for IdHeader {
    fn from(value: Pd3p2VdoIdHeader) -> Self {
        Self {
            vendor: *(*value.vendor).clone(),
            ..Into::into(value)
        }
    }
}

#[cfg(feature = "c_api")]
impl From<IdHeader> for Pd3p2VdoIdHeader {
    fn from(value: IdHeader) -> Self {
        Self {
            vendor: std::mem::ManuallyDrop::new(Box::new(value.vendor.clone())),
            ..Into::into(value)
        }
    }
}

impl FromBytes for IdHeader {
    fn from_bytes(reader: &mut BitReader) -> Result<Self> {
        let usb_host_capability = reader.read_bit()?;
        let usb_device_capability = reader.read_bit()?;

        let sop_product_type_ufp = reader.read(3)?;
        let sop_product_type_ufp =
            SopUfpProductType::n(sop_product_type_ufp).ok_or_else(|| Error::ParseError {
                field: "sop_product_type_ufp".into(),
                value: sop_product_type_ufp,
                #[cfg(feature = "backtrace")]
                backtrace: std::backtrace::Backtrace::capture(),
            })?;

        let modal_operation_supported = reader.read_bit()?;
        let sop_product_type_dfp = reader.read(3)?;
        let sop_product_type_dfp =
            SopDfpProductType::n(sop_product_type_dfp).ok_or_else(|| Error::ParseError {
                field: "sop_product_type_dfp".into(),
                value: sop_product_type_dfp,
                #[cfg(feature = "backtrace")]
                backtrace: std::backtrace::Backtrace::capture(),
            })?;

        let connector_type = reader.read(2)?;
        let connector_type =
            IdHeaderConnectorType::n(connector_type).ok_or_else(|| Error::ParseError {
                field: "connector_type".into(),
                value: connector_type,
                #[cfg(feature = "backtrace")]
                backtrace: std::backtrace::Backtrace::capture(),
            })?;

        reader.skip(5)?;

        let usb_vendor_id = reader.read(16)?;
        let hwdb = udev::Hwdb::new()?;
        let modalias = format!("usb:v{:04X}*", usb_vendor_id);

        let vendor = hwdb
            .query(modalias)
            .next()
            .map_or(std::ffi::OsString::from("Unknown"), |entry| {
                entry.name().to_os_string()
            });

        Ok(IdHeader {
            vendor: vendor.into_string().unwrap_or("Invalid vendor name".into()),
            usb_host_capability,
            usb_device_capability,
            sop_product_type_ufp,
            modal_operation_supported,
            sop_product_type_dfp,
            connector_type,
            usb_vendor_id,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Default, N, Copy, CApiWrapper)]
#[c_api(prefix = "Pd3p2Vdo", repr_c)]
pub enum ProductType {
    /// See USBPDB 6.4.4.3.1.6
    #[default]
    PassiveCableVdo,
    /// See USBPDB 6.4.4.3.1.7
    ActiveCableVdo,
    /// See USBPDB 6.4.4.3.1.9
    VpdVdo,
    /// See USBPDB 6.4.4.3.1.4
    UfpVdo,
    /// See USBPDB 6.4.4.3.1.5
    DfpVdo,
}
