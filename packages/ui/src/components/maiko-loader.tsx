import { useEffect, useState } from "react";
import { cn } from "../lib/cn";
import { MAIKO_FRAME_COUNT, Maiko, type MaikoWay } from "./maiko";

const FRAME_MS = 150;

/** Step through the dance, or hold a single pose when motion is unwelcome. */
export function useDanceFrame(active = true): number | undefined {
  const [frame, setFrame] = useState(0);
  const [animate, setAnimate] = useState(false);

  useEffect(() => {
    if (!active) return;
    const reduced = window.matchMedia?.("(prefers-reduced-motion: reduce)").matches ?? false;
    if (reduced) return;
    setAnimate(true);
    const id = window.setInterval(() => setFrame((f) => (f + 1) % MAIKO_FRAME_COUNT), FRAME_MS);
    return () => window.clearInterval(id);
  }, [active]);

  return animate ? frame : undefined;
}

/**
 * Loading indicator: the maiko dances while the work is in flight.
 * Cherry petals are part of the sprite, so they fall on their own.
 */
export function MaikoLoader({
  label,
  size = 3,
  way = "black",
  className,
}: {
  /** Visible text below the sprite; also used as the accessible status. */
  label?: string;
  /** Pixel size of the sprite in CSS px. */
  size?: number;
  way?: MaikoWay;
  className?: string;
}) {
  const frame = useDanceFrame();
  return (
    <div role="status" aria-live="polite" className={cn("flex flex-col items-center gap-3", className)}>
      <Maiko figure="standing" way={way} frame={frame} size={size} />
      {label && <span className="text-sm text-muted-foreground">{label}</span>}
    </div>
  );
}

/** Compact inline variant for pending buttons: the 16x24 sprite, dancing. */
export function MaikoInline({ size = 1, way = "black", className }: { size?: number; way?: MaikoWay; className?: string }) {
  const frame = useDanceFrame();
  return (
    <span role="status" aria-hidden className={cn("inline-flex items-center align-middle", className)}>
      <Maiko figure="standing" way={way} frame={frame} size={size} small />
    </span>
  );
}

/** The full troupe: black dances centre stage, red and nishiki sit either side. */
export function MaikoTroupe({ size = 2, className }: { size?: number; className?: string }) {
  const frame = useDanceFrame();
  return (
    <div className={cn("flex items-end justify-center gap-2", className)} aria-hidden>
      <Maiko figure="seatedA" way="red" size={size} />
      <Maiko figure="standing" way="black" frame={frame} size={size} />
      <Maiko figure="seatedB" way="nishiki" size={size} />
    </div>
  );
}
