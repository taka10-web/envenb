import { cn } from "../lib/cn";
import { Goldfish } from "./goldfish";

/**
 * Loading indicator: the red, nishiki and black demekin pixel goldfish swim across a short lane.
 * Uses the `animate-swim` / `animate-swim-slow` utilities defined in the host
 * app's Tailwind theme (see apps/desktop/src/index.css). Respects
 * `prefers-reduced-motion` by freezing the fish in place.
 */
export function GoldfishLoader({
  label,
  size = 3,
  className,
}: {
  /** Visible text below the fish; also used as the accessible status. */
  label?: string;
  /** Pixel size of the sprites in CSS px. */
  size?: number;
  className?: string;
}) {
  const laneWidth = 11 * size * 6; // room for roughly six fish lengths
  return (
    <div role="status" aria-live="polite" className={cn("flex flex-col items-center gap-2", className)}>
      <div
        className="relative overflow-hidden"
        style={{ width: laneWidth, height: 7 * size * 3.3, ["--lane" as string]: `${laneWidth}px` }}
        aria-hidden
      >
        <span className="absolute left-0 top-0 animate-swim motion-reduce:animate-none motion-reduce:left-1/3">
          <Goldfish variant="red" size={size} />
        </span>
        <span
          className="absolute left-0 animate-swim-slow motion-reduce:animate-none motion-reduce:left-1/2"
          style={{ top: 7 * size * 1.1 }}
        >
          <Goldfish variant="nishiki" size={size} />
        </span>
        <span
          className="absolute left-0 animate-swim-slower motion-reduce:animate-none motion-reduce:left-2/3"
          style={{ top: 7 * size * 2.2 }}
        >
          <Goldfish variant="demekin" size={size} />
        </span>
      </div>
      {label && <span className="text-sm text-muted-foreground">{label}</span>}
    </div>
  );
}

/** Compact inline variant: one red fish swimming in a lane the height of a line of text. */
export function GoldfishInline({ size = 2, className }: { size?: number; className?: string }) {
  return (
    <span
      role="status"
      aria-hidden
      className={cn("relative inline-block overflow-hidden align-middle", className)}
      style={{ width: 11 * size * 4, height: 7 * size, ["--lane" as string]: `${11 * size * 4}px` }}
    >
      <span className="absolute left-0 top-0 animate-swim motion-reduce:animate-none">
        <Goldfish variant="red" size={size} />
      </span>
    </span>
  );
}
