import { cn } from "../lib/cn";

/**
 * The EnvFish mascots as pixel art: a red goldfish, a nishiki (brocade) one and a
 * black demekin (telescope goldfish with bulging eyes).
 * Rendered as crisp SVG rects from the same 11×8 sprite the CLI uses.
 * Purely decorative (`aria-hidden`).
 */

const RED = [
  "R.....RRRR.",
  "RR...RRRRRR",
  ".RR.RRRRRKR",
  "..RRRRRRRRR",
  ".RR.RRRRRR.",
  "RR...RRRR..",
  "R.....RR...",
  "...........",
];

const NISHIKI = [
  "W.....RRWW.",
  "WR...RWWWWR",
  ".RW.WWRRWGW",
  "..WRWRRWWWW",
  ".WR.WWKKWW.",
  "RW...WWRW..",
  "W.....WW...",
  "...........",
];

const DEMEKIN = [
  "D.....DDDD.",
  "DD...DDDWWD",
  ".DD.DDDDWGD",
  "..DDDDDDDDD",
  ".DD.DDDDDD.",
  "DD...DDDD..",
  "D.....DD...",
  "...........",
];

const PALETTE: Record<string, string> = {
  R: "#e0342e",
  W: "#f4f1ea",
  K: "#1c1917",
  G: "#f5b82e",
  D: "#3f3f46", // readable on both light and dark backgrounds
};

export type GoldfishVariant = "red" | "nishiki" | "demekin";
const SPRITES: Record<GoldfishVariant, string[]> = { red: RED, nishiki: NISHIKI, demekin: DEMEKIN };

export function Goldfish({
  variant = "red",
  size = 4,
  className,
}: {
  variant?: GoldfishVariant;
  /** Pixel size in CSS px. */
  size?: number;
  className?: string;
}) {
  const sprite = SPRITES[variant];
  const w = sprite[0].length;
  const h = sprite.length - 1; // last row is padding
  return (
    <svg
      className={cn("inline-block shrink-0 select-none align-middle", className)}
      width={w * size}
      height={h * size}
      viewBox={`0 0 ${w} ${h}`}
      shapeRendering="crispEdges"
      aria-hidden
    >
      {sprite.slice(0, h).flatMap((row, y) =>
        [...row].map((c, x) =>
          PALETTE[c] ? <rect key={`${x}-${y}`} x={x} y={y} width={1} height={1} fill={PALETTE[c]} /> : null,
        ),
      )}
    </svg>
  );
}
