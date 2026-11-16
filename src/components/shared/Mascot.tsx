import React from "react";

interface MascotProps {
  /** Height in px. */
  size: number;
  /** Wave once on arrival, or high-five (re-key it to play again). */
  animation?: "wave" | "slap" | null;
}

/** Kothalipi's mascot: the speech-bubble mark, optionally animated. */
export const Mascot: React.FC<MascotProps> = ({ size, animation = null }) => (
  <span
    style={{ width: size * (352 / 364), height: size }}
    className={`relative block origin-[50%_90%] select-none ${animation === "wave" ? "animate-wave" : animation === "slap" ? "animate-slap" : ""}`}
  >
    <svg
      viewBox="80 80 352 364"
      className="absolute inset-0 h-full w-full drop-shadow-[0_6px_14px_rgba(0,0,0,0.18)]"
      aria-hidden="true"
    >
      <path
        d="M160 92 H352 Q420 92 420 160 V300 Q420 368 352 368 H222 L132 432 L150 362 Q92 348 92 300 V160 Q92 92 160 92 Z"
        fill="#006A4E"
      />
      <path
        d="M196 132 H316 V190 Q316 262 256 330 Q196 262 196 190 Z"
        fill="#FFFFFF"
      />
      <line
        x1="256"
        y1="246"
        x2="256"
        y2="326"
        stroke="#006A4E"
        strokeWidth="10"
        strokeLinecap="round"
      />
      <circle cx="256" cy="226" r="19" fill="#F42A41" />
    </svg>
  </span>
);
