import { performance } from "node:perf_hooks";

import { formatRoutingSnapshot } from "../src/constants/routeTable.ts";
import { buildIpScanPlan } from "../src/hooks/ipScanPlan.ts";
import { mergeNicDescriptions } from "../src/nicDescriptionModel.ts";
import { validateRouteForm } from "../src/networkValidation.ts";
import type { NetworkInterface, RouteEntry } from "../src/api.ts";

type ProfileResult = {
  avgMs: number;
  iterations: number;
  name: string;
  totalMs: number;
};

function measure(name: string, iterations: number, run: () => void): ProfileResult {
  const start = performance.now();
  for (let index = 0; index < iterations; index += 1) {
    run();
  }
  const totalMs = performance.now() - start;

  return {
    avgMs: totalMs / iterations,
    iterations,
    name,
    totalMs,
  };
}

const sampleNic: NetworkInterface = {
  index: "11",
  ip: "192.168.88.25",
  gateway: "192.168.88.1",
  description: "Intel Test Adapter",
};

const sampleRoutes: RouteEntry[] = Array.from({ length: 256 }, (_, index) => ({
  destination: `10.20.${Math.floor(index / 16)}.${(index % 16) * 16}`,
  netmask: "255.255.255.0",
  gateway: "192.168.88.1",
  metric: "10",
  interface_index: "11",
}));

sampleRoutes.unshift({
  destination: "192.168.88.0",
  netmask: "255.255.255.0",
  gateway: "0.0.0.0",
  metric: "10",
  interface_index: "11",
});

const sampleNics: NetworkInterface[] = Array.from({ length: 128 }, (_, index) => ({
  index: String(index + 1),
  ip: `192.168.${Math.floor(index / 32)}.${(index % 32) + 10}`,
  gateway: "192.168.0.1",
  description: `Ethernet ${index + 1}`,
}));

const descriptionEntries = sampleNics.map((nic, index) => ({
  interfaceIndex: nic.index,
  description: `Adapter ${index + 1} ${nic.description}`,
}));

const results = [
  measure("buildIpScanPlan", 5000, () => {
    buildIpScanPlan(sampleNic, sampleRoutes);
  }),
  measure("mergeNicDescriptions", 5000, () => {
    mergeNicDescriptions(sampleNics, descriptionEntries);
  }),
  measure("formatRoutingSnapshot", 1000, () => {
    formatRoutingSnapshot(sampleRoutes);
  }),
  measure("validateRouteForm", 10000, () => {
    validateRouteForm({
      dest: "10.20.30.0",
      mask: "255.255.255.0",
      gw: "192.168.88.1",
      metric: "10",
    });
  }),
];

console.log("Frontend profiling baseline");
for (const result of results) {
  console.log(
    `${result.name}: ${result.totalMs.toFixed(2)} ms total / ${result.iterations} iterations / ${result.avgMs.toFixed(4)} ms avg`,
  );
}
