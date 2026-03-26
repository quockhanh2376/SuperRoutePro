import { useCallback, useEffect, useMemo, useRef, useState } from "react";

type BufferedLogApi = {
  version: number;
  text: string;
  appendLines: (lines: string[]) => void;
  appendLine: (line: string) => void;
  clear: () => void;
};

export function useBufferedLog(maxLines: number): BufferedLogApi {
  const linesRef = useRef<string[]>([]);
  const rafRef = useRef<number | null>(null);
  const [version, setVersion] = useState(0);

  const scheduleRender = useCallback(() => {
    if (typeof window === "undefined") {
      setVersion((current) => current + 1);
      return;
    }

    if (rafRef.current !== null) return;
    rafRef.current = window.requestAnimationFrame(() => {
      rafRef.current = null;
      setVersion((current) => current + 1);
    });
  }, []);

  const clear = useCallback(() => {
    if (!linesRef.current.length) return;
    linesRef.current = [];
    scheduleRender();
  }, [scheduleRender]);

  const appendLines = useCallback(
    (lines: string[]) => {
      if (!lines.length) return;

      const buffer = linesRef.current;
      buffer.push(...lines);
      if (buffer.length > maxLines) {
        buffer.splice(0, buffer.length - maxLines);
      }
      scheduleRender();
    },
    [maxLines, scheduleRender],
  );

  const appendLine = useCallback(
    (line: string) => {
      appendLines([line]);
    },
    [appendLines],
  );

  const text = useMemo(() => linesRef.current.join("\n"), [version]);

  useEffect(() => {
    return () => {
      if (rafRef.current !== null && typeof window !== "undefined") {
        window.cancelAnimationFrame(rafRef.current);
      }
    };
  }, []);

  return {
    version,
    text,
    appendLines,
    appendLine,
    clear,
  };
}
