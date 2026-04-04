/**
 * Formats battery percentages while keeping missing values readable.
 */
export const formatBatteryPercent = (value: number | null | undefined, fractionDigits = 1): string => {
  if (value === null || value === undefined || Number.isNaN(value)) {
    return "--";
  }
  return `${value.toFixed(fractionDigits)}%`;
};

/**
 * Formats raw battery capacity in mWh for summary cards.
 */
export const formatBatteryCapacity = (value: number | null | undefined): string => {
  if (value === null || value === undefined || Number.isNaN(value)) {
    return "--";
  }
  return `${value.toLocaleString("en-US")} mWh`;
};

/**
 * Converts battery runtime minutes into a short human-readable label.
 */
export const formatBatteryMinutes = (value: number | null | undefined): string => {
  if (value === null || value === undefined || value <= 0) {
    return "--";
  }
  const hours = Math.floor(value / 60);
  const minutes = value % 60;
  if (hours <= 0) {
    return `${minutes} min`;
  }
  return `${hours}h ${minutes}m`;
};

/**
 * Buckets battery wear into a simple support-friendly severity label.
 */
export const getBatteryWearLevel = (wearPercent: number | null | undefined): string => {
  if (wearPercent === null || wearPercent === undefined || Number.isNaN(wearPercent)) {
    return "Unknown";
  }
  if (wearPercent <= 15) return "Good";
  if (wearPercent <= 30) return "Moderate";
  return "High wear";
};
