import { useEffect, type RefObject } from "react";

export function useAutoScroll(ref: RefObject<HTMLElement | null>, version: number): void {
  useEffect(() => {
    if (ref.current) {
      ref.current.scrollTop = ref.current.scrollHeight;
    }
  }, [ref, version]);
}
