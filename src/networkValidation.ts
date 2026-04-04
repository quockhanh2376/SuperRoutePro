export interface RouteValidationInput {
  dest: string;
  mask: string;
  gw: string;
  metric: string;
}

export type RouteValidationErrors = Partial<Record<keyof RouteValidationInput, string>>;
export type RouteDeleteValidationErrors = Partial<Record<"dest" | "mask", string>>;

const IPV4_SEGMENT_COUNT = 4;
const IPV4_OCTET_MAX = 255;
const ROUTE_METRIC_MIN = 1;
const ROUTE_METRIC_MAX = 9999;

/**
 * Parses an IPv4 string into four octets, or returns null when invalid.
 */
export function parseIpv4(value: string): number[] | null {
  const segments = value.trim().split(".");
  if (segments.length !== IPV4_SEGMENT_COUNT) {
    return null;
  }

  const octets = segments.map((segment) => Number.parseInt(segment, 10));
  if (
    octets.some((octet, index) => {
      const segment = segments[index];
      return (
        segment.trim() === "" ||
        !Number.isInteger(octet) ||
        octet < 0 ||
        octet > IPV4_OCTET_MAX
      );
    })
  ) {
    return null;
  }

  return octets;
}

/**
 * Validates a plain IPv4 address.
 */
export function isValidIpv4Address(value: string): boolean {
  return parseIpv4(value) !== null;
}

/**
 * Validates CIDR notation like `10.0.0.0/24`.
 */
export function isValidCidrNotation(value: string): boolean {
  const [address, prefixText] = value.trim().split("/");
  if (!address || !prefixText || !isValidIpv4Address(address)) {
    return false;
  }

  const prefix = Number.parseInt(prefixText, 10);
  return Number.isInteger(prefix) && prefix >= 0 && prefix <= 32;
}

const ipv4ToInt = (octets: number[]): number =>
  (
    ((octets[0] << 24) >>> 0) +
    ((octets[1] << 16) >>> 0) +
    ((octets[2] << 8) >>> 0) +
    (octets[3] >>> 0)
  ) >>> 0;

/**
 * Validates that a subnet mask is a contiguous IPv4 mask.
 */
export function isValidSubnetMask(value: string): boolean {
  const octets = parseIpv4(value);
  if (!octets) {
    return false;
  }

  const mask = ipv4ToInt(octets);
  let zeroSeen = false;
  for (let bit = 31; bit >= 0; bit -= 1) {
    const isOne = ((mask >>> bit) & 1) === 1;
    if (!isOne) {
      zeroSeen = true;
      continue;
    }

    if (zeroSeen) {
      return false;
    }
  }

  return true;
}

/**
 * Checks whether the destination already represents the network address for a mask.
 */
export function isNetworkAddress(destination: string, mask: string): boolean {
  const destinationOctets = parseIpv4(destination);
  const maskOctets = parseIpv4(mask);
  if (!destinationOctets || !maskOctets) {
    return false;
  }

  const destinationInt = ipv4ToInt(destinationOctets);
  const maskInt = ipv4ToInt(maskOctets);
  return (destinationInt & maskInt) === destinationInt;
}

/**
 * Validates a route form and returns field-level errors suitable for inline UI feedback.
 */
export function validateRouteForm(values: RouteValidationInput): RouteValidationErrors {
  const errors: RouteValidationErrors = {};
  const destination = values.dest.trim();
  const mask = values.mask.trim();
  const gateway = values.gw.trim();
  const metricText = values.metric.trim();

  if (!destination) {
    errors.dest = "Destination is required.";
  } else if (!isValidIpv4Address(destination)) {
    errors.dest = "Destination must be a valid IPv4 address.";
  }

  if (!mask) {
    errors.mask = "Subnet mask is required.";
  } else if (!isValidSubnetMask(mask)) {
    errors.mask = "Subnet mask must be a valid contiguous IPv4 mask.";
  }

  if (!gateway) {
    errors.gw = "Gateway is required.";
  } else if (!isValidIpv4Address(gateway)) {
    errors.gw = "Gateway must be a valid IPv4 address.";
  }

  if (!metricText) {
    errors.metric = "Metric is required.";
  } else {
    const metric = Number.parseInt(metricText, 10);
    if (
      !Number.isInteger(metric) ||
      metric < ROUTE_METRIC_MIN ||
      metric > ROUTE_METRIC_MAX
    ) {
      errors.metric = `Metric must be an integer between ${ROUTE_METRIC_MIN} and ${ROUTE_METRIC_MAX}.`;
    }
  }

  if (!errors.dest && !errors.mask && !isNetworkAddress(destination, mask)) {
    errors.dest = "Destination must be the network address for the provided subnet mask.";
  }

  return errors;
}

/**
 * Validates the minimal destination/mask pair used by route deletion.
 */
export function validateRouteDeleteInput(values: {
  dest: string;
  mask: string;
}): RouteDeleteValidationErrors {
  const errors: RouteDeleteValidationErrors = {};
  const destination = values.dest.trim();
  const mask = values.mask.trim();

  if (!destination) {
    errors.dest = "Destination is required.";
  } else if (!isValidIpv4Address(destination)) {
    errors.dest = "Destination must be a valid IPv4 address.";
  }

  if (!mask) {
    errors.mask = "Subnet mask is required.";
  } else if (!isValidSubnetMask(mask)) {
    errors.mask = "Subnet mask must be a valid contiguous IPv4 mask.";
  }

  return errors;
}
