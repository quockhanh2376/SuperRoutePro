import type { NetworkInterface } from "./api";

export type NicDescriptionEntry = {
  interfaceIndex: string;
  description: string;
};

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
    const description = descriptions.get(nic.index);
    if (!description || description === nic.description) {
      return nic;
    }
    return {
      ...nic,
      description,
    };
  });
}
