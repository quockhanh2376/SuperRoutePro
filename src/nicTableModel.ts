type NicTableMessageInput = {
  nicCount: number;
  loading: boolean;
  hasLoadedOnce: boolean;
  activeOnly: boolean;
};

export function getNicTableMessage({
  nicCount,
  loading,
  hasLoadedOnce,
  activeOnly,
}: NicTableMessageInput): string | null {
  if (nicCount > 0) {
    return null;
  }

  if (loading || !hasLoadedOnce) {
    return "Loading interfaces...";
  }

  return activeOnly ? "No active interfaces found" : "No interfaces found";
}
