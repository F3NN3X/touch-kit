import type { CSSProperties } from "react";

interface IconProps {
  name: string;
  size: number;
  fill?: boolean;
  color?: string;
  style?: CSSProperties;
}

export function Icon({ name, size, fill, color, style }: IconProps) {
  return (
    <span
      className="material-symbols-rounded"
      style={{ fontSize: size, color, fontVariationSettings: fill ? "'FILL' 1" : undefined, ...style }}
    >
      {name}
    </span>
  );
}
