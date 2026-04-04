import assert from "node:assert/strict";
import test from "node:test";

import {
  isNetworkAddress,
  isValidCidrNotation,
  isValidIpv4Address,
  isValidSubnetMask,
  validateRouteDeleteInput,
  validateRouteForm,
} from "../src/networkValidation.ts";

test("accepts valid IPv4, CIDR, and subnet masks", () => {
  assert.equal(isValidIpv4Address("192.168.1.1"), true);
  assert.equal(isValidCidrNotation("10.0.0.0/24"), true);
  assert.equal(isValidSubnetMask("255.255.255.0"), true);
});

test("rejects invalid IPv4 values and non-contiguous subnet masks", () => {
  assert.equal(isValidIpv4Address("999.1.1.1"), false);
  assert.equal(isValidCidrNotation("10.0.0.1/99"), false);
  assert.equal(isValidSubnetMask("255.0.255.0"), false);
});

test("route form validation returns field-level errors for invalid input", () => {
  assert.deepEqual(
    validateRouteForm({
      dest: "10.0.0.42",
      mask: "255.255.255.0",
      gw: "bad-gateway",
      metric: "0",
    }),
    {
      dest: "Destination must be the network address for the provided subnet mask.",
      gw: "Gateway must be a valid IPv4 address.",
      metric: "Metric must be an integer between 1 and 9999.",
    },
  );
});

test("route delete validation requires a valid destination and mask", () => {
  assert.deepEqual(
    validateRouteDeleteInput({
      dest: "",
      mask: "255.0.255.0",
    }),
    {
      dest: "Destination is required.",
      mask: "Subnet mask must be a valid contiguous IPv4 mask.",
    },
  );
});

test("detects when a destination is already the network address for its mask", () => {
  assert.equal(isNetworkAddress("10.10.20.0", "255.255.255.0"), true);
  assert.equal(isNetworkAddress("10.10.20.5", "255.255.255.0"), false);
});
