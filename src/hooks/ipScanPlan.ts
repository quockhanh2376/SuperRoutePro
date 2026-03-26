import type { NetworkInterface, RouteEntry } from "../api";

export type IpScanPlan = {
  targets: string[];
  subnetLabel: string;
  truncated: boolean;
  source: "route" | "fallback";
};

const IP_SCAN_MAX_TARGETS = 512;
const FALLBACK_IP_SCAN_PREFIX = 24;

const parseIpv4 = (value: string): number[] | null => {
  const parts = value.trim().split(".");
  if (parts.length !== 4) return null;
  const octets = parts.map((part) => Number.parseInt(part, 10));
  if (octets.some((octet) => !Number.isFinite(octet) || octet < 0 || octet > 255)) {
    return null;
  }
  return octets;
};

const ipv4ToInt = (octets: number[]): number =>
  (
    ((octets[0] << 24) >>> 0) +
    ((octets[1] << 16) >>> 0) +
    ((octets[2] << 8) >>> 0) +
    (octets[3] >>> 0)
  ) >>> 0;

const intToIpv4 = (value: number): string =>
  `${(value >>> 24) & 255}.${(value >>> 16) & 255}.${(value >>> 8) & 255}.${value & 255}`;

const prefixToMaskInt = (prefix: number): number => {
  if (prefix <= 0) return 0;
  if (prefix >= 32) return 0xffffffff >>> 0;
  return (0xffffffff << (32 - prefix)) >>> 0;
};

const maskToPrefix = (mask: string): number | null => {
  const octets = parseIpv4(mask);
  if (!octets) return null;
  const maskInt = ipv4ToInt(octets);
  let prefix = 0;
  let zeroSeen = false;
  for (let bit = 31; bit >= 0; bit -= 1) {
    const isOne = ((maskInt >>> bit) & 1) === 1;
    if (isOne) {
      if (zeroSeen) return null;
      prefix += 1;
    } else {
      zeroSeen = true;
    }
  }
  return prefix;
};

export function buildIpScanPlan(nic: NetworkInterface, routes: RouteEntry[]): IpScanPlan | null {
  const nicOctets = parseIpv4(nic.ip);
  if (!nicOctets) return null;

  const nicInt = ipv4ToInt(nicOctets);
  let networkInt: number | null = null;
  let prefix: number | null = null;
  let source: "route" | "fallback" = "fallback";

  const connectedRoute = routes.find((route) => {
    if (route.interface_index !== nic.index) return false;
    if (route.gateway !== "0.0.0.0") return false;
    if (route.destination === "0.0.0.0" || route.netmask === "255.255.255.255") return false;
    return parseIpv4(route.destination) !== null && parseIpv4(route.netmask) !== null;
  });

  if (connectedRoute) {
    const routePrefix = maskToPrefix(connectedRoute.netmask);
    const routeDestination = parseIpv4(connectedRoute.destination);
    if (
      routePrefix !== null &&
      routePrefix >= 16 &&
      routePrefix <= 30 &&
      routeDestination
    ) {
      const routeMaskInt = prefixToMaskInt(routePrefix);
      networkInt = ipv4ToInt(routeDestination) & routeMaskInt;
      prefix = routePrefix;
      source = "route";
    }
  }

  if (networkInt === null || prefix === null) {
    prefix = FALLBACK_IP_SCAN_PREFIX;
    networkInt = nicInt & prefixToMaskInt(prefix);
    source = "fallback";
  }

  const hostSpan = 2 ** (32 - prefix);
  const hostCapacity = Math.max(0, hostSpan - 2);
  if (hostCapacity <= 0) return null;

  const firstHost = networkInt + 1;
  const lastHost = networkInt + hostSpan - 2;
  const selfInRange = nicInt >= firstHost && nicInt <= lastHost;
  const availableTargets = Math.max(0, hostCapacity - (selfInRange ? 1 : 0));
  const scanCount = Math.min(IP_SCAN_MAX_TARGETS, availableTargets);
  const targets: string[] = [];

  for (let offset = 1; offset < hostSpan - 1 && targets.length < scanCount; offset += 1) {
    const hostInt = (networkInt + offset) >>> 0;
    if (hostInt === nicInt) continue;
    targets.push(intToIpv4(hostInt));
  }

  return {
    targets,
    subnetLabel: `${intToIpv4(networkInt)}/${prefix}`,
    truncated: availableTargets > targets.length,
    source,
  };
}
