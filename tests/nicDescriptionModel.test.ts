import test from "node:test";
import assert from "node:assert/strict";

import {
  choosePreferredNicDescription,
  mergeNicDescriptions,
  stabilizeNicSnapshotDescriptions,
} from "../src/nicDescriptionModel.ts";
import type { NetworkInterface } from "../src/api.ts";

const BASE_NICS: NetworkInterface[] = [
  {
    index: "30",
    ip: "192.168.88.101",
    gateway: "192.168.88.1",
    description: "Ethernet 4",
  },
  {
    index: "6",
    ip: "10.184.1.60",
    gateway: "",
    description: "Ethernet 5",
  },
];

test("replaces generic startup labels with richer adapter descriptions", () => {
  assert.deepEqual(
    mergeNicDescriptions(BASE_NICS, [
      { interfaceIndex: "30", description: "Intel(R) Ethernet Connection I219-LM" },
      { interfaceIndex: "6", description: "Fortinet Virtual Ethernet Adapter" },
    ]),
    [
      {
        ...BASE_NICS[0],
        description: "Intel(R) Ethernet Connection I219-LM",
      },
      {
        ...BASE_NICS[1],
        description: "Fortinet Virtual Ethernet Adapter",
      },
    ],
  );
});

test("ignores empty enrichment values and preserves the snapshot description", () => {
  assert.deepEqual(
    mergeNicDescriptions(BASE_NICS, [
      { interfaceIndex: "30", description: "   " },
      { interfaceIndex: "6", description: "" },
    ]),
    BASE_NICS,
  );
});

test("ignores enrichment rows that do not match any loaded NIC", () => {
  assert.deepEqual(
    mergeNicDescriptions(BASE_NICS, [
      { interfaceIndex: "999", description: "Unused adapter" },
    ]),
    BASE_NICS,
  );
});

test("keeps richer NIC descriptions when a later snapshot regresses to Ethernet aliases", () => {
  assert.deepEqual(
    stabilizeNicSnapshotDescriptions(
      [
        {
          ...BASE_NICS[0],
          description: "Broadcom NetXtreme Gigabit Ethernet",
        },
        {
          ...BASE_NICS[1],
          description: "Realtek PCIe GbE Family Controller",
        },
      ],
      BASE_NICS,
    ),
    [
      {
        ...BASE_NICS[0],
        description: "Broadcom NetXtreme Gigabit Ethernet",
      },
      {
        ...BASE_NICS[1],
        description: "Realtek PCIe GbE Family Controller",
      },
    ],
  );
});

test("prefers incoming richer descriptions over generic aliases", () => {
  assert.equal(
    choosePreferredNicDescription("Ethernet 2", "Broadcom NetXtreme Gigabit Ethernet"),
    "Broadcom NetXtreme Gigabit Ethernet",
  );
});

test("rejects generic replacement when current NIC description is already richer", () => {
  assert.equal(
    choosePreferredNicDescription("Realtek PCIe GbE Family Controller", "Ethernet 3"),
    "Realtek PCIe GbE Family Controller",
  );
});
