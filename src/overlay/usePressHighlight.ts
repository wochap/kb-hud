import { useEffect, useRef, useState } from "react";

const MIN_VISIBLE_MS = 120;
const FADE_MS = 150;

interface KeyTiming {
  downAt: number;
  upAt?: number;
}

function intensityOf(timing: KeyTiming, now: number): number {
  if (timing.upAt === undefined) return 1;
  const fadeStart = Math.max(timing.upAt, timing.downAt + MIN_VISIBLE_MS);
  const progress = (now - fadeStart) / FADE_MS;
  return Math.max(0, 1 - progress);
}

/**
 * Turns the raw pressed-position set into per-key highlight intensities:
 * full while held, at least ~120 ms of total visibility, then a ~150 ms fade.
 */
export function usePressHighlight(pressed: number[]): Map<number, number> {
  const timings = useRef(new Map<number, KeyTiming>());
  const [intensities, setIntensities] = useState<Map<number, number>>(
    () => new Map(),
  );

  const pressedKey = pressed.join(",");

  useEffect(() => {
    const now = performance.now();
    const current = new Set(pressed);
    for (const [position, timing] of timings.current) {
      if (!current.has(position) && timing.upAt === undefined) {
        timing.upAt = now;
      }
    }
    for (const position of current) {
      const existing = timings.current.get(position);
      if (!existing || existing.upAt !== undefined) {
        timings.current.set(position, { downAt: now });
      }
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [pressedKey]);

  useEffect(() => {
    let frame = 0;
    const tick = () => {
      const now = performance.now();
      const next = new Map<number, number>();
      for (const [position, timing] of timings.current) {
        const intensity = intensityOf(timing, now);
        if (intensity > 0) {
          next.set(position, intensity);
        } else {
          timings.current.delete(position);
        }
      }
      setIntensities((prev) => {
        if (prev.size === next.size) {
          let same = true;
          for (const [k, v] of prev) {
            if (next.get(k) !== v) {
              same = false;
              break;
            }
          }
          if (same && next.size === 0) return prev;
        }
        return next;
      });
      if (timings.current.size > 0) {
        frame = requestAnimationFrame(tick);
      }
    };
    frame = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frame);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [pressedKey]);

  return intensities;
}
