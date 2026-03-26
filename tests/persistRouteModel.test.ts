import test from "node:test";
import assert from "node:assert/strict";
import {
  buildPersistCustomRoutes,
  getPersistRouteInterfaceIndexes,
  isPersistableCustomRoute,
} from "../src/persistRouteModel.ts";

test("isPersistableCustomRoute excludes default and on-link routes", () => {
  assert.equal(
    isPersistableCustomRoute({
      destination: "0.0.0.0",
      netmask: "0.0.0.0",
      gateway: "192.168.88.1",
      metric: "1",
      interface_index: "22",
    }),
    false,
  );
  assert.equal(
    isPersistableCustomRoute({
      destination: "192.168.88.0",
      netmask: "255.255.255.0",
      gateway: "On-link",
      metric: "266",
      interface_index: "22",
    }),
    false,
  );
  assert.equal(
    isPersistableCustomRoute({
      destination: "10.184.0.0",
      netmask: "255.255.255.0",
      gateway: "10.184.1.1",
      metric: "60",
      interface_index: "19",
    }),
    true,
  );
});

test("getPersistRouteInterfaceIndexes returns unique interface indexes for persistable routes", () => {
  const indexes = getPersistRouteInterfaceIndexes([
    {
      destination: "10.184.0.0",
      netmask: "255.255.255.0",
      gateway: "10.184.1.1",
      metric: "60",
      interface_index: "19",
    },
    {
      destination: "10.184.7.0",
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
  ]);

  assert.deepEqual(indexes, ["19"]);
});

test("buildPersistCustomRoutes attaches per-route NIC identifiers when available", () => {
  const nicByInterfaceIndex = new Map([
    ["19", { description: "Realtek PCIe GbE Family Controller", mac_address: "E4-54-E8-E3-3A-1A" }],
    ["22", { description: "Broadcom NetXtreme Gigabit Ethernet", mac_address: "00-10-18-1A-EB-2D" }],
  ]);

  const routes = buildPersistCustomRoutes(
    [
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
    ],
    nicByInterfaceIndex,
  );

  assert.deepEqual(routes, [
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
  ]);
});
