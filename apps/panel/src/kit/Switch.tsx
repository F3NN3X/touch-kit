import { caption, C, EASE, ring } from "./tokens";
import { usePress } from "./usePress";

interface SwitchProps {
  label: string;
  /** Caption under the label. */
  hint: string;
  on: boolean;
  onChange: (on: boolean) => void;
}

/** The whole 64 px row is the touch target, not just the track. */
export function Switch({ label, hint, on, onChange }: SwitchProps) {
  const press = usePress();
  return (
    <div
      onPointerDown={press.onPointerDown}
      onClick={() => onChange(!on)}
      style={{
        height: 64,
        borderRadius: 14,
        background: press.pressed ? C.pressed : C.control,
        boxShadow: ring(press.pressed ? C.pressedRing : C.controlRing),
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        gap: 10,
        padding: "0 12px 0 14px",
        transition: "background .12s",
      }}
    >
      <div style={{ display: "flex", flexDirection: "column", gap: 3, minWidth: 0 }}>
        <span style={{ fontSize: 13, fontWeight: 500 }}>{label}</span>
        <span style={{ ...caption, whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>{hint}</span>
      </div>
      <div
        style={{
          position: "relative",
          flex: "none",
          width: 56,
          height: 32,
          borderRadius: 16,
          background: on ? C.accent : C.pressed,
          transition: "background .18s",
        }}
      >
        <div
          style={{
            position: "absolute",
            top: 3,
            left: on ? 27 : 3,
            width: 26,
            height: 26,
            borderRadius: 13,
            background: C.text,
            boxShadow: "0 1px 3px rgba(0,0,0,.4)",
            transition: `left .18s ${EASE}`,
          }}
        />
      </div>
    </div>
  );
}
