/**
 * Converts unknown thrown values into a stable user-facing message.
 */
export function toErrorMessage(error: unknown): string {
  if (error instanceof Error) {
    return error.message || error.name || "Unknown error";
  }

  if (typeof error === "string") {
    const message = error.trim();
    return message || "Unknown error";
  }

  if (error && typeof error === "object" && "message" in error) {
    const message = (error as { message?: unknown }).message;
    if (typeof message === "string" && message.trim()) {
      return message.trim();
    }
  }

  try {
    const serialized = JSON.stringify(error);
    if (serialized && serialized !== "{}") {
      return serialized;
    }
  } catch {
    // Ignore JSON serialization failures and fall back to string coercion.
  }

  return String(error ?? "Unknown error");
}

/**
 * Formats a contextual error message for the footer/status line.
 */
export function formatErrorMessage(action: string, error: unknown): string {
  return `${action}: ${toErrorMessage(error)}`;
}

/**
 * Formats an exception for multiline command output consoles.
 */
export function formatOutputError(error: unknown): string {
  return `Error: ${toErrorMessage(error)}`;
}

/**
 * Provides a consistent success/failure sentence for command handlers.
 */
export function formatActionResultMessage(
  title: string,
  success: boolean,
  options?: { successMessage?: string; failureMessage?: string },
): string {
  if (success) {
    return options?.successMessage ?? `${title} completed successfully.`;
  }

  return options?.failureMessage ?? `${title} failed.`;
}

/**
 * Detects the common Windows elevation errors emitted by netsh/ipconfig helpers.
 */
export function isAdminElevationError(output: string | null | undefined): boolean {
  return /requires elevation|run as administrator|os error 740/i.test(output ?? "");
}

/**
 * Returns the first present validation error from a flat error map.
 */
export function getFirstValidationError(
  errors: Record<string, string | undefined>,
): string | null {
  for (const value of Object.values(errors)) {
    if (value) {
      return value;
    }
  }
  return null;
}
