export type PersistStartupWriteMode = "save" | "clear";

type ResolvePersistStartupEnabledArgs = {
  localPreference: boolean | null;
  legacyTaskEnabled: boolean | null;
  persistedConfigEnabled: boolean | null;
};

export function getPersistStartupWriteMode(enabled: boolean): PersistStartupWriteMode {
  return enabled ? "save" : "clear";
}

export function resolvePersistStartupEnabled({
  localPreference,
  legacyTaskEnabled,
  persistedConfigEnabled,
}: ResolvePersistStartupEnabledArgs): boolean {
  if (persistedConfigEnabled === true || legacyTaskEnabled === true) {
    return true;
  }

  if (persistedConfigEnabled === false || legacyTaskEnabled === false) {
    return false;
  }

  return localPreference ?? false;
}
