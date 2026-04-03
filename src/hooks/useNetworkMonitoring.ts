import { useEffect, useState } from "react";

import { checkInternet, pingHost } from "../api";

interface UseNetworkMonitoringOptions {
  latencyTarget?: string;
}

interface UseNetworkMonitoringResult {
  isOnline: boolean | null;
  currentLatency: number;
}

export function useNetworkMonitoring(
  options: UseNetworkMonitoringOptions = {},
): UseNetworkMonitoringResult {
  const { latencyTarget = "8.8.8.8" } = options;
  const [isOnline, setIsOnline] = useState<boolean | null>(null);
  const [currentLatency, setCurrentLatency] = useState(0);

  useEffect(() => {
    let stopped = false;
    let timerId: number | null = null;
    let inFlight = false;
    let successStreak = 0;
    let failureStreak = 0;

    const computeDelay = (online: boolean): number => {
      if (!online) {
        return Math.min(12000, 2500 + failureStreak * 1200);
      }
      if (successStreak >= 6) return 15000;
      if (successStreak >= 3) return 9000;
      return 5000;
    };

    const scheduleNext = (delayMs: number) => {
      if (stopped) return;
      timerId = window.setTimeout(() => {
        void tick();
      }, delayMs);
    };

    const tick = async () => {
      if (stopped || inFlight) return;
      inFlight = true;
      let online = false;
      try {
        online = await checkInternet();
        if (stopped) return;
        setIsOnline(online);
      } catch {
        if (stopped) return;
        online = false;
        setIsOnline(false);
      } finally {
        if (online) {
          successStreak += 1;
          failureStreak = 0;
        } else {
          failureStreak += 1;
          successStreak = 0;
        }
        inFlight = false;
        scheduleNext(computeDelay(online));
      }
    };

    void tick();
    return () => {
      stopped = true;
      if (timerId !== null) {
        window.clearTimeout(timerId);
      }
    };
  }, []);

  useEffect(() => {
    let stopped = false;
    let timerId: number | null = null;
    let inFlight = false;
    let failureStreak = 0;

    const computeDelay = (success: boolean, latencyMs: number): number => {
      if (!success) {
        return Math.min(7000, 1800 + failureStreak * 700);
      }
      if (latencyMs <= 40) return 5000;
      if (latencyMs <= 90) return 3500;
      if (latencyMs <= 180) return 2500;
      return 1800;
    };

    const scheduleNext = (delayMs: number) => {
      if (stopped) return;
      timerId = window.setTimeout(() => {
        void tick();
      }, delayMs);
    };

    const tick = async () => {
      if (stopped || inFlight) return;
      inFlight = true;
      let success = false;
      let latency = 0;
      try {
        const result = await pingHost(latencyTarget, 1);
        if (stopped) return;
        success = result.success;
        latency = success ? result.latency_ms : 0;
        setCurrentLatency(latency);
      } catch {
        if (stopped) return;
        setCurrentLatency(0);
      } finally {
        failureStreak = success ? 0 : failureStreak + 1;
        inFlight = false;
        scheduleNext(computeDelay(success, latency));
      }
    };

    void tick();
    return () => {
      stopped = true;
      if (timerId !== null) {
        window.clearTimeout(timerId);
      }
    };
  }, [latencyTarget]);

  return {
    isOnline,
    currentLatency,
  };
}
