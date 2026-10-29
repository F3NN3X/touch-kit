import type { ReactNode } from "react";
import { Icon } from "./Icon";
import { C, FONT_MONO, ring } from "./tokens";

interface PanelProps {
  index: string;
  title: string;
  badgeIcon: string;
  badge: string;
  gap?: number;
  children: ReactNode;
}

/** Numbered card with the header row every demo panel shares. */
export function Panel({ index, title, badgeIcon, badge, gap = 14, children }: PanelProps) {
  return (
    <section
      style={{
        display: "flex",
        flexDirection: "column",
        gap,
        padding: 18,
        borderRadius: 16,
        background: C.panel,
        boxShadow: ring(C.panelRing),
        minWidth: 0,
        overflow: "hidden",
      }}
    >
      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", height: 28 }}>
        <div style={{ display: "flex", alignItems: "baseline", gap: 10 }}>
          <span style={{ fontFamily: FONT_MONO, fontSize: 12, color: C.text4 }}>{index}</span>
          <span style={{ fontSize: 17, fontWeight: 600 }}>{title}</span>
        </div>
        <span
          style={{
            display: "flex",
            alignItems: "center",
            gap: 5,
            height: 24,
            padding: "0 9px",
            borderRadius: 999,
            background: C.control,
            fontFamily: FONT_MONO,
            fontSize: 11,
            color: C.text2,
          }}
        >
          <Icon name={badgeIcon} size={15} />
          {badge}
        </span>
      </div>
      {children}
    </section>
  );
}
