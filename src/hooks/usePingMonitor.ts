import { useCallback, useEffect, useRef, useState } from "react";

import { fpingScan, pingHost, type FpingScanResult, type PingResult } from "../api";
import { formatErrorMessage } from "../errorUtils";

type PingMode = "ping" | "fping";

interface UsePingMonitorOptions {
  appendLine: (line: string) => void;
  appendLines: (lines: string[]) => void;
  setStatusMessage: (message: string) => void;
  initialTarget?: string;
  fpingScanFn?: (targets: string[], timeoutMs?: number) => Promise<FpingScanResult>;
  pingHostFn?: (target: string, count?: number) => Promise<PingResult>;
}

interface UsePingMonitorResult {
  pingTarget: string;
  setPingTarget: (target: string) => void;
  pingMode: PingMode;
  setPingMode: (mode: PingMode) => void;
  pingRunning: boolean;
  handleStartPing: () => void;
  handleStopPing: () => void;
}

export function usePingMonitor({
  appendLine,
  appendLines,
  setStatusMessage,
  initialTarget = "1.1.1.1",
  fpingScanFn = fpingScan,
  pingHostFn = pingHost,
}: UsePingMonitorOptions): UsePingMonitorResult {
  const [pingTarget, setPingTarget] = useState(initialTarget);
  const [pingMode, setPingMode] = useState<PingMode>("ping");
  const [pingRunning, setPingRunning] = useState(false);

  const pingLoopRef = useRef<number | null>(null);
  const pingBusyRef = useRef(false);
  const pingSeqRef = useRef(0);

  const handleStartPing = useCallback(() => {
    const target = pingTarget.trim() || initialTarget;
    setPingTarget(target);
    pingSeqRef.current = 0;
    const label = pingMode === "fping" ? "fping-like" : "ping";
    appendLine(`--- Start ${label} continuous check to ${target} ---`);
    setStatusMessage(`${label} ${target} continuously...`);
    setPingRunning(true);
  }, [appendLine, initialTarget, pingMode, pingTarget, setStatusMessage]);

  const handleStopPing = useCallback(() => {
    const target = pingTarget.trim() || initialTarget;
    setPingRunning(false);
    appendLine(`--- Stopped continuous ping to ${target} ---`);
    setStatusMessage("Ping stopped");
  }, [appendLine, initialTarget, pingTarget, setStatusMessage]);

  useEffect(() => {
    if (!pingRunning) {
      if (pingLoopRef.current !== null) {
        window.clearInterval(pingLoopRef.current);
        pingLoopRef.current = null;
      }
      return;
    }

    const target = pingTarget.trim() || initialTarget;
    const parsedTargets = target
      .split(/[\s,;]+/)
      .map((value) => value.trim())
      .filter(Boolean);
    const fpingTargets = parsedTargets.length > 0 ? parsedTargets : [initialTarget];

    const runOnce = async () => {
      if (pingBusyRef.current) return;
      pingBusyRef.current = true;
      try {
        if (pingMode === "fping") {
          const result = await fpingScanFn(fpingTargets, 1200);
          const stamp = new Date().toLocaleTimeString("en-GB");
          pingSeqRef.current += 1;
          const lines: string[] = [
            `[${stamp}] fping-like round=${pingSeqRef.current} sent=${result.sent} recv=${result.received} loss=${result.loss_percent.toFixed(0)}% min/avg/max=${result.min_ms}/${result.avg_ms}/${result.max_ms}ms`,
          ];
          for (const host of result.hosts) {
            if (host.success) {
              lines.push(`  [UP] ${host.target} ${host.latency_ms} ms`);
            } else {
              lines.push(`  [DOWN] ${host.target} timeout`);
            }
          }
          appendLines(lines);
        } else {
          const result = await pingHostFn(target, 1);
          const stamp = new Date().toLocaleTimeString("en-GB");
          pingSeqRef.current += 1;
          if (result.success) {
            appendLine(`[${stamp}] Reply from ${target}: bytes=32 time=${result.latency_ms}ms TTL=52`);
          } else {
            appendLine(`[${stamp}] Request timed out (${target})`);
          }
        }
      } catch (error: unknown) {
        appendLine(
          `[${new Date().toLocaleTimeString("en-GB")}] ${formatErrorMessage("Ping error", error)}`,
        );
      } finally {
        pingBusyRef.current = false;
      }
    };

    void runOnce();
    pingLoopRef.current = window.setInterval(() => {
      void runOnce();
    }, pingMode === "fping" ? 450 : 1000);

    return () => {
      if (pingLoopRef.current !== null) {
        window.clearInterval(pingLoopRef.current);
        pingLoopRef.current = null;
      }
    };
  }, [appendLine, appendLines, fpingScanFn, initialTarget, pingHostFn, pingMode, pingRunning, pingTarget]);

  return {
    pingTarget,
    setPingTarget,
    pingMode,
    setPingMode,
    pingRunning,
    handleStartPing,
    handleStopPing,
  };
}
