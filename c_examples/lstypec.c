// SPDX-License-Identifier: Apache-2.0 OR MIT
// SPDX-FileCopyrightText: © 2024 Google
// Ported from libtypec (Rajaram Regupathy <rajaram.regupathy@gmail.com>)

// Run with:
// cargo run --example lstypec -- backend sysfs
//
// This is an example of how to use the C API. It is similar in nature to
// the lstypec Rust binary.

#include "libtypec-rs.h"
#include <assert.h>
#include <complex.h>
#include <errno.h>
#include <stdio.h>

int c_example_lstypec(unsigned int backend) {
  int ret = 0;
  struct PdPdo *out_pdos = NULL;
  size_t out_npdos = 0;
  size_t out_mem_sz = 0;
  size_t connector_nr;
  unsigned int backend_type = backend ? backend : OsBackends_Sysfs;

  struct TypecRs *typec;
  ret = libtypec_rs_new(backend_type, &typec);
  if (typec == NULL) {
    fprintf(stderr, "Failed to create TypecRs instance\n");
    return ret;
  }

  // Get the capabilities
  struct UcsiCapability capabilities;
  ret = libtypec_rs_get_capabilities(typec, &capabilities);
  if (ret != 0) {
    fprintf(stderr, "Failed to get capabilities\n");
    return ret;
  }

  for (connector_nr = 0; connector_nr < capabilities.num_connectors;
       connector_nr++) {
    // Connector capabilities
    struct UcsiConnectorCapability connector;
    ret = libtypec_rs_get_conn_capabilities(typec, connector_nr, &connector);
    if (ret < 0) {
      fprintf(stderr, "Failed to get connector %zu\n", connector_nr);
      return ret;
    }

    // Connector PDOs (Source)
    ret = libtypec_rs_get_pdos(
        typec, connector_nr, false, 0, 0, UcsiPdoType_Source,
        UcsiPdoSourceCapabilitiesType_CurrentSupportedSourceCapabilities,
        capabilities.pd_version, &out_pdos, &out_npdos, &out_mem_sz);
    if (!ret) {
      assert(out_pdos);
      libtypec_rs_destroy_pdos(out_pdos, out_npdos, out_mem_sz);
    } else if (ret != -ENOTSUP) {
      fprintf(stderr, "Failed to get the Connector Source PDOs %zu\n",
              connector_nr);
      return ret;
    }

    // Connector PDOs (Sink)
    ret = libtypec_rs_get_pdos(
        typec, connector_nr, false, 0, 0, UcsiPdoType_Sink,
        UcsiPdoSourceCapabilitiesType_CurrentSupportedSourceCapabilities,
        capabilities.pd_version, &out_pdos, &out_npdos, &out_mem_sz);
    if (!ret) {
      assert(out_pdos);
      libtypec_rs_destroy_pdos(out_pdos, out_npdos, out_mem_sz);
    } else if (ret != -ENOTSUP) {
      fprintf(stderr, "Failed to get the Connector Sink PDOs %zu\n",
              connector_nr);
      return ret;
    }

    // Cable properties
    struct UcsiCableProperty cable_props;
    ret = libtypec_rs_get_cable_properties(typec, connector_nr, &cable_props);
    if (!ret) {
    } else if (ret != -ENOTSUP) {
      fprintf(stderr, "Failed to get cable properties\n");
      return ret;
    }

    // Supported alternate modes
    struct UcsiAlternateMode *alt_modes;
    size_t nmodes;
    size_t modes_mem_sz;
    ret = libtypec_rs_get_alternate_modes(
        typec, UcsiGetAlternateModesRecipient_Connector, connector_nr, &alt_modes,
        &nmodes, &modes_mem_sz);
    if (!ret) {
      libtypec_rs_destroy_alternate_modes(alt_modes, nmodes, modes_mem_sz);
    } else if (ret != -ENOTSUP) {
      fprintf(stderr, "Failed to get connector alt modes\n");
      return ret;
    }

    // Cable
    ret = libtypec_rs_get_alternate_modes(
        typec, UcsiGetAlternateModesRecipient_SopPrime, connector_nr, &alt_modes,
        &nmodes, &modes_mem_sz);
    if (ret == 0) {
      libtypec_rs_destroy_alternate_modes(alt_modes, nmodes, modes_mem_sz);
    } else if (ret != -ENOTSUP) {
      fprintf(stderr, "Failed to get SOP' alt modes\n");
      return ret;
    }

    struct PdMessage pd_msg;
    ret = libtypec_rs_get_pd_message(
        typec, connector_nr, PdMessageRecipient_SopPrime,
        PdMessageResponseType_DiscoverIdentity, &pd_msg);

    if (!ret) {
      libtypec_rs_destroy_pd_message(&pd_msg);
    } else if (ret != -ENOTSUP) {
      fprintf(stderr,
              "Failed to get the DiscoverIdentity PD message for SOP'\n");
      return ret;
    }

    // Partner
    ret = libtypec_rs_get_alternate_modes(typec, UcsiGetAlternateModesRecipient_Sop,
                                          connector_nr, &alt_modes, &nmodes,
                                          &modes_mem_sz);
    if (!ret) {
      libtypec_rs_destroy_alternate_modes(alt_modes, nmodes, modes_mem_sz);
    } else if (ret != -ENOTSUP) {
      fprintf(stderr, "Failed to get SOP alt modes\n");
      return ret;
    }

    ret = libtypec_rs_get_pd_message(
        typec, connector_nr, PdMessageRecipient_Sop,
        PdMessageResponseType_DiscoverIdentity, &pd_msg);

    if (!ret) {
    } else if (ret != -ENOTSUP) {
      fprintf(stderr,
              "Failed to get the DiscoverIdentity PD message for SOP\n");
      return ret;
    }

    out_pdos = NULL;
    out_npdos = 0;
    out_mem_sz = 0;
    ret = libtypec_rs_get_pdos(
        typec, connector_nr, /*partner=*/true, 0, 0, UcsiPdoType_Source,
        UcsiPdoSourceCapabilitiesType_CurrentSupportedSourceCapabilities,
        capabilities.pd_version, &out_pdos, &out_npdos, &out_mem_sz);
    if (!ret) {
      assert(out_pdos);
      libtypec_rs_destroy_pdos(out_pdos, out_npdos, out_mem_sz);
    } else if (ret != -ENOTSUP) {
      fprintf(stderr, "Failed to get Partner Source PDOs");
      return ret;
    }

    out_pdos = NULL;
    out_npdos = 0;
    out_mem_sz = 0;
    ret = libtypec_rs_get_pdos(
        typec, connector_nr, /*partner=*/true, 0, 0, UcsiPdoType_Sink,
        UcsiPdoSourceCapabilitiesType_CurrentSupportedSourceCapabilities,
        capabilities.pd_version, &out_pdos, &out_npdos, &out_mem_sz);
    if (!ret) {
      assert(out_pdos);
      libtypec_rs_destroy_pdos(out_pdos, out_npdos, out_mem_sz);
    } else if (ret != -ENOTSUP) {
      fprintf(stderr, "Failed to get Partner Sink PDOs");
      return ret;
    }
  }

  // Do not forget to destroy the library instance.
  libtypec_rs_destroy(typec);
  return 0;
}