const trimErrorPrefix = (message: string, prefix: string) => {
  return message.startsWith(prefix) ? message.slice(prefix.length).trim() : message;
};

export function formatSpeedTestError(error: unknown): string {
  const rawMessage = error instanceof Error ? error.message : String(error);
  const message = rawMessage.trim();
  const lower = message.toLowerCase();

  if (lower.includes("timed out") || lower.includes("timeout")) {
    return "Speed test timed out while talking to the test server. Check internet, VPN, or firewall and try again.";
  }

  if (lower.includes("latency probe")) {
    return `Latency check failed before the speed test could start. ${trimErrorPrefix(message, "Latency probe failed:")}`;
  }

  if (lower.includes("download stream failed") || lower.includes("download test")) {
    return `Download stage failed. ${trimErrorPrefix(message, "Download test failed to start:")}`;
  }

  if (lower.includes("upload test")) {
    return `Upload stage failed. ${trimErrorPrefix(message, "Upload test failed to start:")}`;
  }

  if (lower.includes("could not initialize speed test client")) {
    return "Speed test could not initialize its network client. Try restarting the app and run it again.";
  }

  if (lower.includes("connection refused") || lower.includes("dns error") || lower.includes("connect")) {
    return "Speed test could not reach the test server. Check internet, VPN, proxy, or firewall and try again.";
  }

  return message;
}
