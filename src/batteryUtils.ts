export const formatBatteryPercent = (value: number | null | undefined, fractionDigits = 1): string => {
  if (value === null || value === undefined || Number.isNaN(value)) {
    return "--";
  }
  return `${value.toFixed(fractionDigits)}%`;
};

export const formatBatteryCapacity = (value: number | null | undefined): string => {
  if (value === null || value === undefined || Number.isNaN(value)) {
    return "--";
  }
  return `${value.toLocaleString("en-US")} mWh`;
};

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

export const getBatteryWearLevel = (wearPercent: number | null | undefined): string => {
  if (wearPercent === null || wearPercent === undefined || Number.isNaN(wearPercent)) {
    return "Unknown";
  }
  if (wearPercent <= 15) return "Good";
  if (wearPercent <= 30) return "Moderate";
  return "High wear";
};
