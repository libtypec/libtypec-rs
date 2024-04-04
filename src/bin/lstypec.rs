// SPDX-License-Identifier: Apache-2.0 OR MIT
// SPDX-FileCopyrightText: © 2024 Google
// Ported from libtypec (Rajaram Regupathy <rajaram.regupathy@gmail.com>)

//! Implements listing of typec port and port partner details

use argh::FromArgs;

use libtypec_rs::pd::MessageRecipient;
use libtypec_rs::pd::MessageResponseType;
use libtypec_rs::typec::OsBackends;
use libtypec_rs::typec::TypecRs;
use libtypec_rs::ucsi::GetAlternateModesRecipient;
use libtypec_rs::ucsi::PdoSourceCapabilitiesType;
use libtypec_rs::ucsi::PdoType;
use libtypec_rs::Error;

#[derive(FromArgs)]
/// List typec port and port partner details
struct Args {
    /// enable verbose mode
    #[argh(switch, short = 'v')]
    _verbose: bool,
    /// the backend to use
    #[argh(option)]
    backend: Option<OsBackends>,
}

fn main() {
    let args: Args = argh::from_env();

    let backends = if let Some(backend) = args.backend {
        // Use the backend selected by the user
        vec![backend]
    } else {
        // Try the backends in the order given by the array.
        [OsBackends::Sysfs, OsBackends::UcsiDebugfs].into()
    };

    let mut typec = backends
        .iter()
        .find_map(|backend| TypecRs::new(*backend).ok())
        .expect("No valid backend found");

    let capabilities = typec.capabilities().expect("Failed to get capabilities");
    println!("USB-C Platform Policy Manager Capability");
    println!("{:#?}", capabilities);
    println!();

    for connector_nr in 0..capabilities.num_connectors {
        let conn_capability = typec
            .connector_capabilties(connector_nr)
            .expect("Failed to get connector capabilities");

        println!("Connector {connector_nr} Capability/Status");
        println!("{:#?}", conn_capability);
        println!();

        match typec.pdos(
            connector_nr,
            false,
            0,
            0,
            PdoType::Source,
            PdoSourceCapabilitiesType::CurrentSupportedSourceCapabilities,
            capabilities.pd_version,
        ) {
            Ok(conn_pdo) => {
                println!("Connector {connector_nr} Source PDOs");
                println!("{:#?}", conn_pdo);
                println!();
            }
            Err(Error::UnsupportedUsbRevision { revision, .. }) => {
                println!("Unsupported USB revision {:?} in Source PDOs", revision)
            }
            Err(e) => panic!("Failed to get source PDOs for {connector_nr}: {:?}", e),
        }

        match typec.pdos(
            connector_nr,
            false,
            0,
            0,
            PdoType::Sink,
            PdoSourceCapabilitiesType::CurrentSupportedSourceCapabilities,
            capabilities.pd_version,
        ) {
            Ok(conn_pdo) => {
                println!("Connector {connector_nr} Sink PDOs");
                println!("{:#?}", conn_pdo);
                println!();
            }
            Err(Error::NotSupported { .. }) => {}
            Err(Error::UnsupportedUsbRevision { revision, .. }) => {
                println!("Unsupported USB revision {:?} in sink PDOs", revision)
            }
            Err(e) => panic!("Failed to get sink PDOs for {connector_nr}: {:?}", e),
        }

        match typec.cable_properties(connector_nr) {
            Ok(cable_props) => {
                println!("Connector {connector_nr} Cable Properties");
                println!("{:#?}", cable_props);
            }
            Err(libtypec_rs::Error::NotSupported { .. }) => {
                println!("No cable identified for {connector_nr}");
            }
            Err(e) => panic!("Failed to get cable properties for {connector_nr}: {:?}", e),
        }
        println!();

        let alternate_modes = typec
            .alternate_modes(GetAlternateModesRecipient::Connector, connector_nr)
            .expect("Failed to get alternate modes");

        println!("Connector {connector_nr} Alternate Modes");
        println!("{:#?}", alternate_modes);
        println!();

        let alternate_modes = typec
            .alternate_modes(GetAlternateModesRecipient::SopPrime, connector_nr)
            .expect("Failed to get alternate modes");

        println!("Connector {connector_nr} SOP' Alternate Modes");
        println!("{:#?}", alternate_modes);
        println!();

        match typec.pd_message(
            connector_nr,
            MessageRecipient::Sop,
            MessageResponseType::DiscoverIdentity,
        ) {
            Ok(pd_message) => {
                println!("Connector {connector_nr} SOP DiscoverIdentity PD Message");
                println!("{:#?}", pd_message);
            }
            Err(Error::NotSupported { .. }) => {}
            Err(e) => panic!(
                "Failed to get the DiscoverIdentity PD Message for SOP {:?}",
                e
            ),
        };
        println!();

        let alternate_modes = typec
            .alternate_modes(GetAlternateModesRecipient::Sop, connector_nr)
            .expect("Failed to get alternate modes");

        println!("Connector {connector_nr} SOP' Alternate Modes");
        println!("{:#?}", alternate_modes);
        println!();

        match typec.pd_message(
            connector_nr,
            MessageRecipient::SopPrime,
            MessageResponseType::DiscoverIdentity,
        ) {
            Ok(pd_message) => {
                println!("Connector {connector_nr} SOP' DiscoverIdentity PD Message");
                println!("{:#?}", pd_message);
            }
            Err(Error::NotSupported { .. }) => {}
            Err(e) => panic!(
                "Failed to get the DiscoverIdentity PD Message for SOP' {:?}",
                e
            ),
        };
        println!();

        match typec.pdos(
            connector_nr,
            true,
            0,
            0,
            PdoType::Source,
            PdoSourceCapabilitiesType::CurrentSupportedSourceCapabilities,
            capabilities.pd_version,
        ) {
            Ok(conn_pdo) => {
                println!("Partner PDO data (Source)");
                println!("{:#?}", conn_pdo);
            }
            Err(Error::NotSupported { .. }) => {}
            Err(Error::UnsupportedUsbRevision { revision, .. }) => {
                println!("Unsupported USB revision {:?} in source PDOs", revision)
            }
            Err(e) => panic!("Failed to get Source PDOs {:?}", e),
        }
        println!();

        match typec.pdos(
            connector_nr,
            true,
            0,
            0,
            PdoType::Sink,
            PdoSourceCapabilitiesType::CurrentSupportedSourceCapabilities,
            capabilities.pd_version,
        ) {
            Ok(conn_pdo) => {
                println!("Partner PDO data (Sink)");
                println!("{:#?}", conn_pdo);
            }
            Err(Error::NotSupported { .. }) => {}
            Err(Error::UnsupportedUsbRevision { revision, .. }) => {
                println!("Unsupported USB revision {:?} in Sink PDOs", revision)
            }
            Err(e) => panic!("Failed to get Sink PDOs {:?}", e),
        }
        println!();
    }
}
