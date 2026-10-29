import type { ReactNode } from "react";
import { Icon } from "./Icon";
import { C, ring } from "./tokens";
import type { PinchZoom } from "./usePinchZoom";

interface PinchSurfaceProps {
  pinch: PinchZoom;
  /** Zooms and pans with the gesture. */
  children: ReactNode;
  /** Stays put on top of the zoomed content. */
  overlay?: ReactNode;
}

export function PinchSurface({ pinch, children, overlay }: PinchSurfaceProps) {
  return (
    <div
      {...pinch.surface}
      style={{
        position: "relative",
        flex: 1,
        minHeight: 0,
        borderRadius: 14,
        background: C.well,
        boxShadow: ring(C.panelRing),
        overflow: "hidden",
        touchAction: "none",
      }}
    >
      <div
        ref={pinch.contentRef}
        style={{
          position: "absolute",
          inset: "44px 16px 30px",
          transform: pinch.transform,
          transformOrigin: "50% 50%",
          transition: pinch.pinching ? "none" : "transform .2s ease-out",
        }}
      >
        {children}
      </div>
      {overlay}
      {pinch.zoom < 1.02 && (
        <div
          style={{
            position: "absolute",
            left: "50%",
            top: "50%",
            transform: "translate(-50%, -50%)",
            display: "flex",
            alignItems: "center",
            gap: 6,
            height: 34,
            padding: "0 14px",
            borderRadius: 999,
            background: "rgba(19,19,22,.92)",
            boxShadow: ring(C.controlRing),
            fontSize: 12,
            color: C.text2,
            whiteSpace: "nowrap",
            pointerEvents: "none",
          }}
        >
          <Icon name="pinch" size={18} />
          Pinch or scroll
        </div>
      )}
    </div>
  );
}
