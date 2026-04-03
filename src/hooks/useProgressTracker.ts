import { useState, useCallback } from "react";

/**
 * Custom hook for managing progress tracking state.
 * Eliminates the duplicate pattern of:
 *   const [progressPercent, setProgressPercent] = useState(0);
 *   const [progressText, setProgressText] = useState("Ready.");
 * 
 * Used by BloatwareModal, CacheModal, and IpScanModal for tracking
 * long-running operations with progress updates.
 * 
 * @param initialText - Initial progress text (default: "Ready.")
 * @returns Object with percent, text, and update/reset functions
 * 
 * @example
 * // Simple usage
 * const progress = useProgressTracker();
 * progress.update(50, "Processing item 5/10...");
 * progress.reset();
 * 
 * // Custom initial text
 * const scanProgress = useProgressTracker("Idle.");
 * 
 * // Access state
 * <div>{progress.percent}% - {progress.text}</div>
 */
export function useProgressTracker(initialText = "Ready.") {
  const [percent, setPercent] = useState(0);
  const [text, setText] = useState(initialText);

  /**
   * Update both progress percent and text together
   */
  const update = useCallback((newPercent: number, newText: string) => {
    setPercent(newPercent);
    setText(newText);
  }, []);

  /**
   * Update only the progress percent
   */
  const setProgress = useCallback((newPercent: number) => {
    setPercent(newPercent);
  }, []);

  /**
   * Update only the progress text
   */
  const setMessage = useCallback((newText: string) => {
    setText(newText);
  }, []);

  /**
   * Reset progress to initial state (0%, initial text)
   */
  const reset = useCallback(() => {
    setPercent(0);
    setText(initialText);
  }, [initialText]);

  return {
    percent,
    text,
    update,
    setProgress,
    setMessage,
    reset,
  };
}
