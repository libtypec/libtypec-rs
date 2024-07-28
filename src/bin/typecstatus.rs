// SPDX-License-Identifier: Apache-2.0 OR MIT
// SPDX-FileCopyrightText: © 2024 Google
// Ported from libtypec (Rajaram Regupathy <rajaram.regupathy@gmail.com>)

//! Check status of TypeC ports

use libtypec_rs::backends::sysfs::sysfs_reader::SysfsReader;
use libtypec_rs::typec::OsBackends;
use libtypec_rs::typec::TypecRs;

fn read_power_contract() {
    let mut typec = TypecRs::new(OsBackends::Sysfs).expect("Failed to get a library instance");
    let capabilities = typec.capabilities().expect("Failed to get capabilities");

    println!("USB-C Power Status:");
    println!("Number of USB-C port(s): {}", capabilities.num_connectors);

    for connector_nr in 0..capabilities.num_connectors {
        let conn_status = typec
            .connector_status(connector_nr)
            .expect("Failed to get connector capabilities");

        if conn_status.negotiated_power_level > 0 {
            let operating_power =
                (((conn_status.negotiated_power_level >> 10) & 0x3ff) * 250) / 1000;
            let max_power = ((conn_status.negotiated_power_level & 0x3ff) * 250) / 1000;

            println!(
                "\tUSB-C power contract Operating Power {} W, with Max Power {} W\n",
                operating_power, max_power
            );

            let mut reader = SysfsReader::new().unwrap();
            reader
                .set_path("/sys/class/powercap/intel-rapl:0/constraint_0_power_limit_uw")
                .unwrap();
            let tdp = reader.read_u32().unwrap() / 1000000;
            reader
                .set_path("/sys/class/powercap/intel-rapl:0/constraint_1_power_limit_uw")
                .unwrap();
            let bst_pwr = reader.read_u32().unwrap() / 1000000;

            println!(
                "\tCharging System with TDP {} W, with boost power requirement of {} W\n",
                tdp, bst_pwr
            );
        } else {
            println!("\tNo Power Contract on port {connector_nr}",);
        }
    }
}

fn main() {
    read_power_contract();
}
