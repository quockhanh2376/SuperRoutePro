import test from "node:test";
import assert from "node:assert/strict";

import type { NetworkInterface, NicIdentifier, PersistConfig, RouteEntry } from "../src/api.ts";
import { mergeNicDescriptions, type NicDescriptionEntry } from "../src/nicDescriptionModel.ts";
import {
  buildPersistCustomRoutes,
  getPersistRouteInterfaceIndexes,
  isPersistableCustomRoute,
} from "../src/persistRouteModel.ts";
import {
  getPersistStartupWriteMode,
  resolvePersistStartupEnabled,
} from "../src/persistStartupModel.ts";

function buildPersistConfigFromSnapshot(args: {
  selectedNic: NetworkInterface;
  routes: RouteEntry[];
  stableIds: NicIdentifier[];
}): PersistConfig {
  const { selectedNic, routes, stableIds } = args;
  const persistRouteInterfaceIndexes = getPersistRouteInterfaceIndexes(routes);
  const stableIdIndexes = Array.from(new Set([selectedNic.index, ...persistRouteInterfaceIndexes]));
  const routeNicEntries = new Map(
    [
      [selectedNic.index, stableIds[0]] as const,
      ...stableIdIndexes.slice(1).map((interfaceIndex, index) => [
        interfaceIndex,
        stableIds[index + 1],
      ] as const),
    ],
  );

  return {
    schema_version: 1,
    enabled: true,
    nic: stableIds[0],
    wan: { gateway: selectedNic.gateway, metric: "1" },
    custom_routes: buildPersistCustomRoutes(routes, routeNicEntries),
    updated_at: "2026-03-26T08:20:00.000Z",
  };
}

test("network snapshot enrich feeds persist config with stable NIC metadata", () => {
  const snapshotNics: NetworkInterface[] = [
    { index: "19", ip: "10.184.1.44", gateway: "10.184.1.1", description: "Ethernet" },
    { index: "22", ip: "192.168.88.10", gateway: "192.168.88.1", description: "Wi-Fi" },
    { index: "24", ip: "", gateway: "", description: "Offline adapter" },
  ];
  const descriptionEntries: NicDescriptionEntry[] = [
    { interfaceIndex: "19", description: "Realtek PCIe GbE Family Controller" },
    { interfaceIndex: "22", description: "  Intel(R) Wi-Fi 6 AX201 160MHz  " },
    { interfaceIndex: "24", description: "" },
  ];
  const routes: RouteEntry[] = [
    {
      destination: "10.184.0.0",
      netmask: "255.255.255.0",
      gateway: "10.184.1.1",
      metric: "60",
      interface_index: "19",
    },
    {
      destination: "192.168.88.0",
      netmask: "255.255.255.0",
      gateway: "On-link",
      metric: "266",
      interface_index: "22",
    },
    {
      destination: "172.16.0.0",
      netmask: "255.240.0.0",
      gateway: "172.16.1.1",
      metric: "20",
      interface_index: "22",
    },
  ];

  const merged = mergeNicDescriptions(snapshotNics, descriptionEntries);

  assert.deepEqual(merged, [
    {
      index: "19",
      ip: "10.184.1.44",
      gateway: "10.184.1.1",
      description: "Realtek PCIe GbE Family Controller",
    },
    {
      index: "22",
      ip: "192.168.88.10",
      gateway: "192.168.88.1",
      description: "Intel(R) Wi-Fi 6 AX201 160MHz",
    },
    {
      index: "24",
      ip: "",
      gateway: "",
      description: "Offline adapter",
    },
  ]);

  const config = buildPersistConfigFromSnapshot({
    selectedNic: merged[0],
    routes,
    stableIds: [
      { description: "Realtek PCIe GbE Family Controller", mac_address: "E4-54-E8-E3-3A-1A" },
      { description: "Intel(R) Wi-Fi 6 AX201 160MHz", mac_address: "D8-9E-F3-11-22-33" },
    ],
  });

  assert.deepEqual(config.nic, {
    description: "Realtek PCIe GbE Family Controller",
    mac_address: "E4-54-E8-E3-3A-1A",
  });
  assert.deepEqual(config.wan, { gateway: "10.184.1.1", metric: "1" });
  assert.deepEqual(config.custom_routes, [
    {
      destination: "10.184.0.0",
      mask: "255.255.255.0",
      gateway: "10.184.1.1",
      metric: "60",
      nic: {
        description: "Realtek PCIe GbE Family Controller",
        mac_address: "E4-54-E8-E3-3A-1A",
      },
    },
    {
      destination: "172.16.0.0",
      mask: "255.240.0.0",
      gateway: "172.16.1.1",
      metric: "20",
      nic: {
        description: "Intel(R) Wi-Fi 6 AX201 160MHz",
        mac_address: "D8-9E-F3-11-22-33",
      },
    },
  ]);
  assert.equal(isPersistableCustomRoute(routes[0]), true);
  assert.equal(isPersistableCustomRoute(routes[1]), false);
});

test("startup preference resolves persisted state before local checkbox state", () => {
  assert.equal(
    resolvePersistStartupEnabled({
      localPreference: false,
      legacyTaskEnabled: false,
      persistedConfigEnabled: true,
    }),
    true,
  );
  assert.equal(
    resolvePersistStartupEnabled({
      localPreference: false,
      legacyTaskEnabled: true,
      persistedConfigEnabled: false,
    }),
    true,
  );
  assert.equal(
    resolvePersistStartupEnabled({
      localPreference: true,
      legacyTaskEnabled: false,
      persistedConfigEnabled: false,
    }),
    false,
  );
  assert.equal(getPersistStartupWriteMode(true), "save");
  assert.equal(getPersistStartupWriteMode(false), "clear");
});
