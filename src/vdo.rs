// SPDX-License-Identifier: Apache-2.0 OR MIT
// SPDX-FileCopyrightText: © 2024 Google
// Ported from libtypec (Rajaram Regupathy <rajaram.regupathy@gmail.com>)

//! The VDO data structures

use proc_macros::CApiWrapper;

use crate::pd::pd3p2::vdo::CertStat as Pd3p2VdoCertStat;
use crate::pd::pd3p2::vdo::Dfp as Pd3p2VdoDfp;
use crate::pd::pd3p2::vdo::IdHeader as Pd3p2VdoIdHeader;
use crate::pd::pd3p2::vdo::ProductType as Pd3p2VdoProductType;
use crate::pd::pd3p2::vdo::Ufp as Pd3p2VdoUfp;
use crate::pd::pd3p2::vdo::Vpd as Pd3p2VdoVpd;

#[cfg(feature = "c_api")]
mod c_api {
    pub(crate) use crate::pd::pd3p2::vdo::Pd3p2VdoCertStat;
    pub(crate) use crate::pd::pd3p2::vdo::Pd3p2VdoDfp;
    pub(crate) use crate::pd::pd3p2::vdo::Pd3p2VdoIdHeader;
    pub(crate) use crate::pd::pd3p2::vdo::Pd3p2VdoProductType;
    pub(crate) use crate::pd::pd3p2::vdo::Pd3p2VdoUfp;
    pub(crate) use crate::pd::pd3p2::vdo::Pd3p2VdoVpd;
}

#[derive(Debug, Clone, PartialEq, CApiWrapper)]
#[c_api(prefix = "TypeCRs", repr_c)]
/// A type representing the different types of VDO supported by the library.
pub enum Vdo {
    Pd3p2IdHeader(Pd3p2VdoIdHeader),
    Pd3p2CertStat(Pd3p2VdoCertStat),
    Pd3p2ProductType(Pd3p2VdoProductType),
    Pd3p2Vpd(Pd3p2VdoVpd),
    Pd3p2Ufp(Pd3p2VdoUfp),
    Pd3p2Dfp(Pd3p2VdoDfp),
}
