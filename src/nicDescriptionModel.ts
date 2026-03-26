import type { NetworkInterface } from "./api";

export type NicDescriptionEntry = {
  interfaceIndex: string;
  description: string;
};

const GENERIC_NIC_DESCRIPTION_PATTERNS = [
  /^ethernet(?:\s+\d+)?$/i,
  /^wi-?fi(?:\s+\d+)?$/i,
  /^wlan(?:\s+\d+)?$/i,
  /^local area connection(?:\s+\d+)?$/i,
  /^bluetooth network connection(?:\s+\d+)?$/i,
];

function normalizeDescription(description: string): string {
  return description.trim();
}

export function isGenericNicDescription(description: string): boolean {
  const normalized = normalizeDescription(description);
  if (!normalized) {
    return true;
  }

  return GENERIC_NIC_DESCRIPTION_PATTERNS.some((pattern) => pattern.test(normalized));
}

export function choosePreferredNicDescription(current: string, next: string): string {
  const normalizedCurrent = normalizeDescription(current);
  const normalizedNext = normalizeDescription(next);

  if (!normalizedCurrent) {
    return normalizedNext;
  }
  if (!normalizedNext) {
    return normalizedCurrent;
  }

  const currentIsGeneric = isGenericNicDescription(normalizedCurrent);
  const nextIsGeneric = isGenericNicDescription(normalizedNext);

  if (!currentIsGeneric && nextIsGeneric) {
    return normalizedCurrent;
  }

  return normalizedNext;
}

export function stabilizeNicSnapshotDescriptions(
  previousNics: NetworkInterface[],
  nextNics: NetworkInterface[],
): NetworkInterface[] {
  const previousByIndex = new Map(previousNics.map((nic) => [nic.index, nic]));

  return nextNics.map((nic) => {
    const previous = previousByIndex.get(nic.index);
    if (!previous) {
      return nic;
    }

    const description = choosePreferredNicDescription(previous.description, nic.description);
    if (description === nic.description) {
      return nic;
    }

    return {
      ...nic,
      description,
    };
  });
}

export function syncSelectedNicToList(
  selectedNic: NetworkInterface | null,
  nics: NetworkInterface[],
): NetworkInterface | null {
  if (!selectedNic) {
    return selectedNic;
  }

  return nics.find((nic) => nic.index === selectedNic.index) ?? selectedNic;
}

export function mergeNicDescriptions(
  nics: NetworkInterface[],
  entries: NicDescriptionEntry[],
): NetworkInterface[] {
  const descriptions = new Map<string, string>();

  for (const entry of entries) {
    const description = entry.description.trim();
    if (!description) {
      continue;
    }
    descriptions.set(entry.interfaceIndex, description);
  }

  return nics.map((nic) => {
    const incomingDescription = descriptions.get(nic.index);
    if (!incomingDescription) {
      return nic;
    }

    const description = choosePreferredNicDescription(nic.description, incomingDescription);
    if (description === nic.description) {
      return nic;
    }

    return {
      ...nic,
      description,
    };
  });
}
