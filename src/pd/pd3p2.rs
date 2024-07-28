// SPDX-License-Identifier: Apache-2.0 OR MIT
// SPDX-FileCopyrightText: © 2024 Google
// Ported from libtypec (Rajaram Regupathy <rajaram.regupathy@gmail.com>)

//! USB Power Delivery 3.2 (PD3.2) functionality.

use bitstream_io::BitRead;
use enumn::N;
use proc_macros::CApiWrapper;

use crate::pd::VdmHeader;
use crate::Error;
use crate::FromBytes;
use crate::Milliamp;
use crate::Millivolt;
use crate::Milliwatt;
use crate::Result;

use crate::pd::pd3p2::vdo::CertStat;
use crate::pd::pd3p2::vdo::IdHeader;
use crate::pd::pd3p2::vdo::Product;
use crate::pd::pd3p2::vdo::ProductType;

#[cfg(feature = "c_api")]
pub(crate) mod c {
    pub(crate) use crate::pd::pd3p2::vdo::Pd3p2VdoCertStat;
    pub(crate) use crate::pd::pd3p2::vdo::Pd3p2VdoIdHeader;
    pub(crate) use crate::pd::pd3p2::vdo::Pd3p2VdoProduct;
    pub(crate) use crate::pd::pd3p2::vdo::Pd3p2VdoProductType;
    pub(crate) use crate::pd::PdVdmHeader;
}

#[cfg(feature = "c_api")]
pub(crate) use c::*;

pub mod vdo;

#[derive(Debug, Clone, PartialEq, Default, CApiWrapper)]
#[c_api(prefix = "Pd3p2", repr_c, manual_from_impl)]
/// The response to a Discover Identity command.
pub struct DiscoverIdentityResponse {
    #[c_api(rename_type = "PdVdmHeader")]
    pub header: VdmHeader,
    #[c_api(rename_type = "Pd3p2VdoIdHeader")]
    pub id_header_vdo: IdHeader,
    #[c_api(rename_type = "Pd3p2VdoCertStat")]
    pub cert_stat: CertStat,
    #[c_api(rename_type = "Pd3p2VdoProduct")]
    pub product_vdo: Product,
    #[c_api(rename_type = "[Pd3p2VdoProductType; 3]")]
    pub product_type_vdo: [ProductType; 3],
}

#[cfg(feature = "c_api")]
impl From<Pd3p2DiscoverIdentityResponse> for DiscoverIdentityResponse {
    fn from(value: Pd3p2DiscoverIdentityResponse) -> Self {
        Self {
            header: value.header.into(),
            id_header_vdo: value.id_header_vdo.into(),
            cert_stat: value.cert_stat.into(),
            product_vdo: value.product_vdo.into(),
            product_type_vdo: value
                .product_type_vdo
                .into_iter()
                .map(ProductType::from)
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
        }
    }
}

#[cfg(feature = "c_api")]
impl From<DiscoverIdentityResponse> for Pd3p2DiscoverIdentityResponse {
    fn from(value: DiscoverIdentityResponse) -> Self {
        Self {
            header: value.header.into(),
            id_header_vdo: value.id_header_vdo.into(),
            cert_stat: value.cert_stat.into(),
            product_vdo: value.product_vdo.into(),
            product_type_vdo: value
                .product_type_vdo
                .into_iter()
                .map(Pd3p2VdoProductType::from)
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, N, Copy, CApiWrapper)]
#[c_api(prefix = "Pd3p2", repr_c)]
pub enum SceTouchTemp {
    #[default]
    NotApplicable = 0,
    Iec60950_1 = 1,
    Iec62368_1Ts1 = 2,
    Iec62368_1Ts2 = 3,
}

#[derive(Debug, Clone, PartialEq, Default, CApiWrapper)]
#[c_api(prefix = "Pd3p2", repr_c)]
pub struct SceLoadStep {
    /// 150mA/µs Load Step (default)
    pub load_step_150ma: bool,
    /// 500mA/µs Load Step
    pub load_step_500ma: bool,
}

#[derive(Debug, Clone, PartialEq, Default, CApiWrapper)]
#[c_api(prefix = "Pd3p2", repr_c)]
pub struct SinkLoadCharacteristics {
    /// Percent overload in 10% increments. Values higher than 25 (11001b)
    /// are clipped to 250%. 00000b is the default.
    pub percent_overload: bool,
    /// Overload period in 20ms when bits 0-4 non-zero.
    pub overload_period: bool,
    /// Duty cycle in 5% increments when bits 0-4 are non-zero
    pub duty_cycle: bool,
    /// Can tolerate VBUS Voltage droop
    pub vbus_voltage_droop: bool,
}

#[derive(Debug, Clone, PartialEq, Default, CApiWrapper)]
#[c_api(prefix = "Pd3p2", repr_c)]
pub struct SCEDCompliance {
    /// Requires LPS Source when set
    pub requires_lps_source: bool,
    /// Requires PS1 Source when set
    pub requires_ps1_source: bool,
    /// Requires PS2 Source when set
    pub requires_ps2_source: bool,
}

#[derive(Debug, Clone, PartialEq, Default, CApiWrapper)]
#[c_api(prefix = "Pd3p2", repr_c)]
pub struct SCEDSinkModes {
    /// 1: PPS charging supported
    pub pps_charging_supported: bool,
    /// 1: VBUS powered
    pub vbus_powered: bool,
    /// 1: Mains powered
    pub mains_powered: bool,
    /// 1: Battery powered
    pub battery_powered: bool,
    /// 1: Battery essentially unlimited
    pub battery_essentially_unlimited: bool,
    /// 1: AVS Supported
    pub avs_supported: bool,
}

#[derive(Debug, Clone, PartialEq, Default, CApiWrapper)]
#[c_api(prefix = "Pd3p2", repr_c)]
pub struct SinkCapabilitiesExtended {
    /// Numeric Vendor ID (assigned by the USB-IF)
    pub vid: u32,
    /// Numeric Product ID (assigned by the manufacturer)
    pub pid: u32,
    /// Numeric Value provided by the USB-IF assigned to the product
    pub xid: u32,
    /// Numeric Firmware version number
    pub fw_version: u32,
    /// Numeric Hardware version number
    pub hw_version: u32,
    /// Numeric SKEDB Version (not the specification Version): Version 1.0 = 1
    pub skedb_version: u32,
    /// Load Step
    pub load_step: SceLoadStep,
    /// Sink Load Characteristics
    pub sink_load_characteristics: SinkLoadCharacteristics,
    /// Compliance
    pub compliance: SCEDCompliance,
    /// Touch Temperature conforms to:
    pub touch_temp: SceTouchTemp,
    /// Battery Info
    pub battery_info: u32,
    /// Sink Modes
    pub sink_modes: SCEDSinkModes,
    /// Sink Minimum PDP
    pub sink_minimum_pdp: u32,
    /// Sink Operational PDP
    pub sink_operational_pdp: u32,
}

#[derive(Debug, Clone, PartialEq, Default, CApiWrapper)]
#[c_api(prefix = "Pd3p2", repr_c)]
pub struct SceVoltageRegulation {
    /// 00b: 150mA/µs Load Step (default)
    pub load_step_150ma: bool,
    /// 01b: 500mA/µs Load Step
    pub load_step_500ma: bool,
    /// 0b: 25% IoC (default)
    pub ioc_25_percent: bool,
    /// 1b: 90% IoC
    pub ioc_90_percent: bool,
}

#[derive(Debug, Clone, PartialEq, Default, CApiWrapper)]
#[c_api(prefix = "Pd3p2", repr_c)]
pub struct SceCompliance {
    /// LPS compliant when set
    pub lps_compliant: bool,
    /// PS1 compliant when set
    pub ps1_compliant: bool,
    /// PS2 compliant when set
    pub ps2_compliant: bool,
}

#[derive(Debug, Clone, PartialEq, Default, CApiWrapper)]
#[c_api(prefix = "Pd3p2", repr_c)]
pub struct SceTouchCurrent {
    /// Low touch Current EPS when set
    pub low_touch_current_eps: bool,
    /// Ground pin supported when set
    pub ground_pin_supported: bool,
    /// Ground pin intended for protective earth when set
    pub ground_pin_for_protective_earth: bool,
}

#[derive(Debug, Clone, PartialEq, Default, CApiWrapper)]
#[c_api(prefix = "Pd3p2", repr_c)]
pub struct ScePeakCurrent {
    /// Percent overload in 10% increments. Values higher than 25 (11001b)
    /// are clipped to 250%.
    pub percent_overload: bool,
    /// Overload period in 20ms
    pub overload_period: bool,
    /// Duty cycle in 5% increments
    pub duty_cycle: bool,
    /// VBUS Voltage droop
    pub vbus_voltage_droop: bool,
}

#[derive(Debug, Clone, PartialEq, Default, CApiWrapper)]
#[c_api(prefix = "Pd3p2", repr_c)]
pub struct SceSourceInputs {
    /// No external supply when set
    pub no_external_supply: bool,
    /// External supply is constrained when set
    pub external_supply_constrained: bool,
    /// Internal battery is present when set
    pub internal_battery_present: bool,
}

#[derive(Debug, Clone, PartialEq, Default, CApiWrapper)]
#[c_api(prefix = "Pd3p2", repr_c)]
pub struct SourceCapabilitiesExtended {
    /// Numeric Vendor ID (assigned by the USB-IF)
    pub vid: u32,
    /// Numeric Product ID (assigned by the manufacturer)
    pub pid: u32,
    /// Numeric Value provided by the USB-IF assigned to the product
    pub xid: u32,
    /// Numeric Firmware version number
    pub fw_version: u32,
    /// Numeric Hardware version number
    pub hw_version: u32,
    /// Voltage Regulation
    pub voltage_regulation: SceVoltageRegulation,
    /// Holdup Time
    pub holdup_time: u32,
    /// Compliance
    pub compliance: SceCompliance,
    /// Touch Current
    pub touch_current: SceTouchCurrent,
    /// Peak Current1
    pub peak_current1: ScePeakCurrent,
    /// Peak Current2
    pub peak_current2: ScePeakCurrent,
    /// Peak Current3
    pub peak_current3: ScePeakCurrent,
    /// Touch Temperature conforms to:
    pub touch_temp: SceTouchTemp,
    /// Source Inputs
    pub source_inputs: SceSourceInputs,
    /// Number of Batteries/Battery Slots
    pub num_batteries_slots: u32,
    /// SPR Source PDP Rating
    pub spr_source_pdp_rating: u32,
    /// EPR Source PDP Rating
    pub epr_source_pdp_rating: u32,
}

/// See USPD - 6.5.3 Get_Battery_Cap Message
#[derive(Debug, Clone, PartialEq, Default, CApiWrapper)]
#[c_api(prefix = "Pd3p2", repr_c)]
pub struct BatteryCapData {
    #[c_api(no_prefix)]
    pub batteries_fixed: [u32; 4],
    #[c_api(no_prefix)]
    pub batteries_hotswappable: [u32; 4],
}

#[derive(Debug, Clone, PartialEq, Default, CApiWrapper)]
#[c_api(prefix = "Pd3p2", repr_c)]
pub struct BSDBatteryInfo {
    /// Invalid Battery reference
    pub invalid_battery_reference: bool,
    /// Battery is present when set
    pub battery_present: bool,
    /// Battery is Charging.
    pub battery_charging: bool,
    /// Battery is Discharging.
    pub battery_discharging: bool,
    /// Battery is Idle.
    pub battery_idle: bool,
}

#[derive(Debug, Clone, PartialEq, Default, CApiWrapper)]
#[c_api(prefix = "Pd3p2", repr_c)]
pub struct BatteryStatusData {
    /// Battery’s State of Charge (SoC) in 0.1 WH increments
    /// Note: 0xFFFF = Battery’s SOC unknown
    pub battery_present_capacity: u32,
    /// Battery Info
    pub battery_info: BSDBatteryInfo,
}

#[derive(Debug, Clone, PartialEq, Default, CApiWrapper)]
#[c_api(prefix = "Pd3p2", repr_c)]
pub struct RevisionMessageData {
    /// Revision.major
    pub revision_major: u32,
    /// Revision.minor
    pub revision_minor: u32,
    /// Version.major
    pub version_major: u32,
    /// Version.minor
    pub version_minor: u32,
    /// Reserved, Shall be set to zero
    pub reserved: u32,
}

#[derive(Debug, Clone, PartialEq, Default, N, Copy, CApiWrapper)]
#[c_api(prefix = "Pd3p2", repr_c)]
/// See USB PD 3.2 - Table 6.17 “Fixed Supply PDO – Sink”
pub enum FastRoleSwap {
    #[default]
    NotSupported,
    DefaultUsbPower,
    OnePointFiveAAtFiveV,
    ThreeAAtFiveV,
}

#[derive(Debug, Clone, PartialEq, Default, CApiWrapper)]
#[c_api(prefix = "Pd3p2", repr_c)]
/// See USB PD 3.2 - Table 6.17 “Fixed Supply PDO – Sink”
pub struct FixedSupplyPdo {
    pub dual_role_power: bool,
    pub higher_capability: bool,
    pub unconstrained_power: bool,
    pub usb_communications_capable: bool,
    pub dual_role_data: bool,
    pub fast_role_swap: FastRoleSwap,
    pub voltage: Millivolt,
    pub operational_current: Milliamp,
}

impl FromBytes for FixedSupplyPdo {
    fn from_bytes(reader: &mut crate::BitReader) -> Result<Self>
    where
        Self: Sized,
    {
        let _ = reader.read::<u32>(2)?; // Fixed supply
        let dual_role_power = reader.read_bit()?;
        let higher_capability = reader.read_bit()?;
        let unconstrained_power = reader.read_bit()?;
        let usb_communications_capable = reader.read_bit()?;
        let dual_role_data = reader.read_bit()?;
        let fast_role_swap_bits = reader.read::<u32>(2)?;
        let fast_role_swap =
            FastRoleSwap::n(fast_role_swap_bits).ok_or_else(|| Error::ParseError {
                field: "fast_role_swap".into(),
                value: fast_role_swap_bits,
                #[cfg(feature = "backtrace")]
                backtrace: std::backtrace::Backtrace::capture(),
            })?;
        let voltage = (reader.read::<u32>(10)? / 50).into();
        let operational_current = (reader.read::<u32>(10)? / 10).into();

        Ok(Self {
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

#[derive(Debug, Clone, PartialEq, Default, CApiWrapper)]
#[c_api(prefix = "Pd3p2", repr_c)]
pub struct BatterySupplyPdo {
    pub max_voltage: Millivolt,
    pub min_voltage: Millivolt,
    pub operational_power: Milliwatt,
}

impl FromBytes for BatterySupplyPdo {
    fn from_bytes(bit_reader: &mut crate::BitReader) -> Result<Self>
    where
        Self: Sized,
    {
        let _ = bit_reader.read::<u32>(2)?; // Battery
        let max_voltage = (bit_reader.read::<u32>(10)? / 50).into();
        let min_voltage = (bit_reader.read::<u32>(10)? / 50).into();
        let operational_power = (bit_reader.read::<u32>(10)? / 10).into();

        Ok(Self {
            max_voltage,
            min_voltage,
            operational_power,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Default, CApiWrapper)]
#[c_api(prefix = "Pd3p2", repr_c)]
pub struct VariableSupplyPdo {
    pub max_voltage: Millivolt,
    pub min_voltage: Millivolt,
    pub max_current: Milliamp,
}

impl FromBytes for VariableSupplyPdo {
    fn from_bytes(reader: &mut crate::BitReader) -> Result<Self>
    where
        Self: Sized,
    {
        let _ = reader.read::<u32>(2)?; // Variable supply
        let max_voltage = (reader.read::<u32>(10)? / 50).into();
        let min_voltage = (reader.read::<u32>(10)? / 50).into();
        let max_current = (reader.read::<u32>(10)? / 10).into();

        Ok(Self {
            max_voltage,
            min_voltage,
            max_current,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Default, CApiWrapper)]
#[c_api(prefix = "Pd3p2", repr_c)]
pub struct SprProgrammableSupplyPdo {
    pub max_voltage: Millivolt,
    pub min_voltage: Millivolt,
    pub max_current: Milliamp,
}

impl FromBytes for SprProgrammableSupplyPdo {
    fn from_bytes(reader: &mut crate::BitReader) -> Result<Self>
    where
        Self: Sized,
    {
        let _ = reader.read::<u32>(2)?; // APDO.
        let _ = reader.read::<u32>(2)?; // Programmable power supply
        let _ = reader.read_bit()?; // PPS power limited
        let _reserved1 = reader.read::<u32>(2)?;
        let max_voltage = (reader.read::<u32>(8)? / 50).into();
        let _reserved2 = reader.read_bit()?;
        let min_voltage = (reader.read::<u32>(8)? / 50).into();
        let _reserved3 = reader.read_bit()?;
        let max_current = (reader.read::<u32>(7)? / 10).into();

        Ok(Self {
            max_voltage,
            min_voltage,
            max_current,
        })
    }
}
