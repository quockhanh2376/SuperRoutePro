export type PersistStartupWriteMode = "save" | "clear";

type ResolvePersistStartupEnabledArgs = {
  localPreference: boolean | null;
  persistedConfigEnabled: boolean | null;
};

export function getPersistStartupWriteMode(enabled: boolean): PersistStartupWriteMode {
  return enabled ? "save" : "clear";
}

export function resolvePersistStartupEnabled({
  localPreference,
  persistedConfigEnabled,
}: ResolvePersistStartupEnabledArgs): boolean {
  if (persistedConfigEnabled === true) {
    return true;
  }

  if (persistedConfigEnabled === false) {
    return false;
  }

  return localPreference ?? false;
}
