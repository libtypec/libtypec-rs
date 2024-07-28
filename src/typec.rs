// SPDX-License-Identifier: Apache-2.0 OR MIT
// SPDX-FileCopyrightText: © 2024 Google
// Ported from libtypec (Rajaram Regupathy <rajaram.regupathy@gmail.com>)

//! The main interface for the library

#[cfg(feature = "c_api")]
use std::mem::ManuallyDrop;
use std::str::FromStr;

use crate::backends;
use crate::pd::Message;
use crate::pd::MessageRecipient;
use crate::pd::MessageResponseType;
use crate::pd::Pdo;
use crate::ucsi::AlternateMode;
use crate::ucsi::CableProperty;
use crate::ucsi::Capability;
use crate::ucsi::ConnectorCapability;
use crate::ucsi::ConnectorStatus;
use crate::ucsi::GetAlternateModesRecipient;
use crate::ucsi::PdoSourceCapabilitiesType;
use crate::ucsi::PdoType;
use crate::BcdWrapper;
#[cfg(feature = "c_api")]
use crate::CError;
use crate::Error;
use crate::OsBackend;
use crate::Result;

/// The main library struct.
/// # Examples
///
/// ```
/// use libtypec_rs::TypecRs;
/// use libtypec_rs::OsBackends;
///
/// let typec = TypecRs::new(OsBackends::Ucsi);
/// assert!(typec.is_ok());
/// ```
pub struct TypecRs {
    /// The OS backend used for this instance.
    os_backend: Box<dyn OsBackend>,
}

/// The OS backends supported by the library.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub enum OsBackends {
    /// A sysfs backend.
    Sysfs,
    /// A UCSI debugfs backend.
    UcsiDebugfs,
}

impl FromStr for OsBackends {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "sysfs" => Ok(Self::Sysfs),
            "ucsi_debugfs" => Ok(Self::UcsiDebugfs),
            _ => Err(Error::NotSupported {
                #[cfg(feature = "backtrace")]
                backtrace: std::backtrace::Backtrace::capture(),
            }),
        }
    }
}

// The Rust API.
impl TypecRs {
    /// Initializes the library with the given `backend`.
    pub fn new(backend: OsBackends) -> Result<Self> {
        match backend {
            OsBackends::UcsiDebugfs => Ok(Self {
                os_backend: Box::new(backends::ucsi_debugfs::UcsiDebugfsBackend::new()?),
            }),
            OsBackends::Sysfs => Ok(Self {
                os_backend: Box::new(backends::sysfs::SysfsBackend::new()?),
            }),
        }
    }

    /// Returns the platform policy capabilities.
    pub fn capabilities(&mut self) -> Result<Capability> {
        self.os_backend.capabilities()
    }

    /// Returns the capability of connector `connector_nr`
    pub fn connector_capabilties(&mut self, connector_nr: usize) -> Result<ConnectorCapability> {
        self.os_backend.connector_capabilties(connector_nr)
    }

    /// Returns the alternate modes that the connector/cable/attached device is
    /// able to support.
    ///
    /// # Arguments
    /// `recipient` Represents alternate mode to be retrieved from local, SOP,
    /// SOP' or SOP"
    /// `connector_nr` The connector number to query.
    pub fn alternate_modes(
        &mut self,
        recipient: GetAlternateModesRecipient,
        connector_nr: usize,
    ) -> Result<Vec<AlternateMode>> {
        self.os_backend.alternate_modes(recipient, connector_nr)
    }

    /// Returns the cable properties of `connector_nr`.
    pub fn cable_properties(&mut self, connector_nr: usize) -> Result<CableProperty> {
        self.os_backend.cable_properties(connector_nr)
    }

    /// Returns the connector status for `connector_nr`.
    pub fn connector_status(&mut self, connector_nr: usize) -> Result<ConnectorStatus> {
        self.os_backend.connector_status(connector_nr)
    }

    /// Get a USB PD message.
    ///
    /// # Arguments
    /// `connector_nr` The connector number to query.
    /// `recipient` Represents the PD message to be retrieved from local, SOP,
    /// SOP' or SOP"
    /// `response_type` Represents the type of response to be retrieved.
    pub fn pd_message(
        &mut self,
        connector_nr: usize,
        recipient: MessageRecipient,
        response_type: MessageResponseType,
    ) -> Result<Message> {
        self.os_backend
            .pd_message(connector_nr, recipient, response_type)
    }

    #[allow(clippy::too_many_arguments)]
    /// Get PDOs from local and partner Policy Managers.
    ///
    /// #Arguments
    ///
    /// `connector_nr` Represents connector to be queried
    /// `partner_pdo` Whether to retrieve partner PDOs
    /// `pdo_offset` Index from which PDO needs to be retrieved
    /// `nr_pdos` Represents number of PDOs to be retrieved
    /// `pdo_type` Whether to retrieve source or sink PDOs
    /// `source_capabilities_type` Represents the type of Source PDOs requested.
    /// `revision` Indicates the USB PD revision used to interpret the read
    /// data.
    pub fn pdos(
        &mut self,
        connector_nr: usize,
        partner_pdo: bool,
        pdo_offset: u32,
        nr_pdos: usize,
        pdo_type: PdoType,
        source_capabilities_type: PdoSourceCapabilitiesType,
        revision: BcdWrapper,
    ) -> Result<Vec<Pdo>> {
        self.os_backend.pdos(
            connector_nr,
            partner_pdo,
            pdo_offset,
            nr_pdos,
            pdo_type,
            source_capabilities_type,
            revision,
        )
    }
}

// The C API.
#[cfg(feature = "c_api")]
impl TypecRs {
    #[no_mangle]
    /// Initializes the library given a `backend`.
    ///
    /// # Arguments
    /// `backend` The backend to use, see `OsBackends` for available options.
    /// `out_typec` An opaque pointer that gets initialized with the new
    /// `TypecRs` instance.
    ///
    /// # Safety
    /// The caller must call libtypec_rs_destroy() at a later point to free up
    /// any allocated resources.
    ///
    /// # Returns
    /// 0 on success, -errno on failure.
    extern "C" fn libtypec_rs_new(
        backend: OsBackends,
        out_typec: *mut *mut Self,
    ) -> std::ffi::c_int {
        match Self::new(backend) {
            Ok(t) => {
                unsafe { *out_typec = Box::into_raw(Box::new(t)) };
                0
            }
            Err(err) => {
                unsafe { *out_typec = std::ptr::null_mut() };
                -CError::from(err).0
            }
        }
    }

    #[no_mangle]
    /// Destroys the `typec` instance.
    ///
    /// # Safety
    /// Must be called with a pointer that was previously acquired from
    /// libtypec_rs_new().
    extern "C" fn libtypec_rs_destroy(typec: &mut Self) {
        let _ = unsafe { Box::from_raw(typec) };
    }

    #[no_mangle]
    /// Returns the platform policy capabilities in `out_capabilities`
    /// and 0 on success, -errno on failure.
    ///
    /// # Safety
    /// The caller must ensure that out_capabilities is a valid pointer.
    extern "C" fn libtypec_rs_get_capabilities(
        &mut self,
        out_capabilities: &mut crate::ucsi::UcsiCapability,
    ) -> std::ffi::c_int {
        match self.capabilities() {
            Ok(cap) => {
                *out_capabilities = cap.into();
                0
            }
            Err(err) => -CError::from(err).0,
        }
    }

    #[no_mangle]
    /// Returns the connector capabilities in `out_conn_capabilities`
    /// and 0 on success, -errno on failure.
    ///
    /// # Arguments
    /// `connector_nr` The connector number to query.
    ///
    /// # Safety
    /// The caller must ensure that out_conn_capabilities is a valid pointer.
    extern "C" fn libtypec_rs_get_conn_capabilities(
        &mut self,
        connector_nr: usize,
        out_conn_capabilities: &mut crate::ucsi::UcsiConnectorCapability,
    ) -> std::ffi::c_int {
        match self.connector_capabilties(connector_nr) {
            Ok(cap) => {
                *out_conn_capabilities = cap.into();
                0
            }
            Err(err) => -CError::from(err).0,
        }
    }

    #[no_mangle]
    /// Returns the alternate modes that the connector/cable/attached device is
    /// able to support in in `out_alternate_modes` and 0 on success, -errno on
    /// failure.
    ///
    /// # Arguments
    /// `recipient` Represents alternate mode to be retrieved from local or SOP
    /// or SOP' or SOP"
    /// `connector_nr` The connector number to query.
    ///
    /// # Safety
    /// The caller must ensure that `out_modes`, `out_nmodes` and `out_mem_sz`
    /// are valid pointers. The caller must call
    /// libtypec_rs_destroy_alternate_modes to free the memory allocated for
    /// `out_modes` at a later point.
    extern "C" fn libtypec_rs_get_alternate_modes(
        &mut self,
        recipient: crate::ucsi::UcsiGetAlternateModesRecipient,
        connector_nr: usize,
        out_modes: *mut *mut crate::ucsi::UcsiAlternateMode,
        out_nmodes: &mut usize,
        out_mem_sz: &mut usize,
    ) -> std::ffi::c_int {
        match self.alternate_modes(recipient.into(), connector_nr) {
            Ok(modes) => {
                let modes: Vec<crate::ucsi::UcsiAlternateMode> =
                    modes.into_iter().map(Into::into).collect();
                *out_nmodes = modes.len();
                *out_mem_sz = modes.capacity();
                unsafe { *out_modes = modes.leak().as_mut_ptr() };
                0
            }
            Err(err) => -CError::from(err).0,
        }
    }

    #[no_mangle]
    /// Frees the memory returned by libtypec_rs_get_alternate_modes.
    ///
    /// # Safety
    /// The caller must ensure that `modes`, `nmodes` and `mem_sz` are pointers
    /// that were returned from a previous call to
    /// libtypec_rs_get_alternate_modes().
    extern "C" fn libtypec_rs_destroy_alternate_modes(
        modes: *mut crate::ucsi::UcsiAlternateMode,
        nmodes: usize,
        mem_sz: usize,
    ) {
        let _ = unsafe { Vec::from_raw_parts(modes, nmodes, mem_sz) };
    }

    #[no_mangle]
    /// Returns the cable property of a connector in `out_cable_properties` and
    /// 0 on success, -errno on failure.
    ///
    /// # Arguments
    /// `connector_nr` The connector number to query.
    ///
    /// # Safety
    /// The caller must ensure that out_cable_properties is a valid pointer.
    extern "C" fn libtypec_rs_get_cable_properties(
        &mut self,
        connector_nr: usize,
        out_cable_properties: &mut crate::ucsi::UcsiCableProperty,
    ) -> std::ffi::c_int {
        match self.cable_properties(connector_nr) {
            Ok(props) => {
                *out_cable_properties = props.into();
                0
            }
            Err(err) => -CError::from(err).0,
        }
    }

    #[no_mangle]
    /// Returns the connector status in `out_connector_status` and 0 on success,
    /// -errno on failure.
    ///
    /// # Arguments
    /// `connector_nr` The connector number to query.
    ///
    /// # Safety
    /// The caller must ensure that out_connector_status is a valid pointer.
    extern "C" fn libtypec_rs_get_connector_status(
        &mut self,
        connector_nr: usize,
        out_connector_status: &mut crate::ucsi::UcsiConnectorStatus,
    ) -> std::ffi::c_int {
        match self.connector_status(connector_nr) {
            Ok(status) => {
                *out_connector_status = status.into();
                0
            }
            Err(err) => -CError::from(err).0,
        }
    }

    #[no_mangle]
    /// Returns the USB PD response message in `out_pd_message` and 0 on
    /// success, -errno on failure.
    ///
    /// # Arguments
    /// `connector_nr` the connector number to retrieve the message from.
    ///
    /// `recipient` represents alternate mode to be retrieved from local or SOP
    /// or SOP' or SOP"
    ///
    /// `response_type` indicates the type of response to be retrieved.
    ///
    /// # Safety
    /// The caller must ensure that out_connector_status is a valid pointer.
    extern "C" fn libtypec_rs_get_pd_message(
        &mut self,
        connector_nr: usize,
        recipient: crate::pd::PdMessageRecipient,
        response_type: crate::pd::PdMessageResponseType,
        out_pd_message: &mut crate::pd::PdMessage,
    ) -> std::ffi::c_int {
        match self.pd_message(connector_nr, recipient.into(), response_type.into()) {
            Ok(msg) => {
                *out_pd_message = msg.into();
                0
            }
            Err(err) => -CError::from(err).0,
        }
    }

    #[no_mangle]
    extern "C" fn libtypec_rs_destroy_pd_message(pd_message: &mut crate::pd::PdMessage) {
        if let crate::pd::PdMessage::Pd3p2DiscoverIdentityResponse(m) = pd_message {
            unsafe { ManuallyDrop::drop(&mut m.id_header_vdo.vendor) }
        }
    }

    #[no_mangle]
    /// Gets PDOs from local and partner Policy Managers
    ///
    /// # Arguments
    /// `conn_num` Represents connector to be queried
    /// `partner`` Set to true to retrieve partner PDOs
    /// `offset` Index from which PDO needs to be retrieved
    /// `num_pdo` Represents number of PDOs to be retrieved
    /// `src_or_sink_pdos` controls whether Source or Sink PDOs are requested
    /// `source_capabilities_type` represents the type of Source PDOs requested
    ///
    /// # Safety
    /// The caller must ensure that `out_pdos`, `out_npdos` and `out_mem_sz` are
    /// valid pointers. The caller must call libtypec_rs_destroy_pdos to free the
    /// memory allocated for `out_pdos` at a later point.
    ///
    /// # Returns
    /// Returns 0 on success, -errno on failures.
    extern "C" fn libtypec_rs_get_pdos(
        &mut self,
        connector_nr: usize,
        partner_pdo: bool,
        pdo_offset: u32,
        nr_pdos: usize,
        src_or_sink_pdos: crate::ucsi::UcsiPdoType,
        source_capabilities_type: crate::ucsi::UcsiPdoSourceCapabilitiesType,
        revision: BcdWrapper,
        out_pdos: *mut *mut crate::pd::PdPdo,
        out_npdos: &mut usize,
        out_mem_sz: &mut usize,
    ) -> std::ffi::c_int {
        match self.pdos(
            connector_nr,
            partner_pdo,
            pdo_offset,
            nr_pdos,
            src_or_sink_pdos.into(),
            source_capabilities_type.into(),
            revision,
        ) {
            Ok(pdos) => {
                let pdos: Vec<crate::pd::PdPdo> = pdos.into_iter().map(Into::into).collect();
                *out_npdos = pdos.len();
                *out_mem_sz = pdos.capacity();
                unsafe { *out_pdos = pdos.leak().as_mut_ptr() };
                0
            }
            Err(err) => -CError::from(err).0,
        }
    }

    #[no_mangle]
    /// Frees the memory returned by libtypec_rs_get_pdos.
    ///
    /// # Safety
    /// The caller must ensure that `pdos`, `npdos` and `mem_sz` are pointers
    /// that were returned from a previous call to libtypec_rs_get_pdos().
    extern "C" fn libtypec_rs_destroy_pdos(
        pdos: *mut crate::pd::PdPdo,
        npdos: usize,
        mem_sz: usize,
    ) {
        let _ = unsafe { Vec::from_raw_parts(pdos, npdos, mem_sz) };
    }
}
