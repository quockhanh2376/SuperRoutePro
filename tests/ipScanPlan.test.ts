import assert from "node:assert/strict";
import test from "node:test";

import { buildIpScanPlan } from "../src/hooks/ipScanPlan.ts";
import type { NetworkInterface, RouteEntry } from "../src/api.ts";

const NIC: NetworkInterface = {
  index: "7",
  ip: "192.168.88.10",
  gateway: "192.168.88.1",
  description: "Primary LAN",
};

test("buildIpScanPlan prefers the connected route subnet when available", () => {
  const routes: RouteEntry[] = [
    {
      destination: "192.168.88.0",
      netmask: "255.255.255.0",
      gateway: "0.0.0.0",
      metric: "10",
      interface_index: "7",
    },
  ];

  const plan = buildIpScanPlan(NIC, routes);

  assert.ok(plan);
  assert.equal(plan?.subnetLabel, "192.168.88.0/24");
  assert.equal(plan?.source, "route");
  assert.equal(plan?.targets.includes("192.168.88.10"), false);
});

test("buildIpScanPlan falls back to a /24 derived from the NIC when no usable route exists", () => {
  const plan = buildIpScanPlan(NIC, []);

  assert.ok(plan);
  assert.equal(plan?.subnetLabel, "192.168.88.0/24");
  assert.equal(plan?.source, "fallback");
});
