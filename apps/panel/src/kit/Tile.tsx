import { Icon } from "./Icon";
import { C, ring } from "./tokens";
import { usePress } from "./usePress";

export type TileState = "rest" | "active" | "alert" | "disabled" | "busy";

interface TileProps {
  icon: string;
  label: string;
  state?: TileState;
  onClick?: () => void;
}

export function Tile({ icon, label, state = "rest", onClick }: TileProps) {
  const inert = state === "disabled" || state === "busy";
  // The pressed look shows in every state but disabled (spec §2), busy included.
  const press = usePress(state === "disabled");
  const down = press.pressed && state !== "disabled";

  let background: string = C.control;
  let shadow: string | undefined = ring(C.controlRing);
  let color: string = state === "active" ? C.accentInk : state === "alert" ? C.red : C.text;
  if (down) {
    background = C.pressed;
    shadow = ring(C.pressedRing);
    color = C.text;
  } else if (state === "active") {
    background = C.accent;
    shadow = undefined;
    color = C.accentInk;
  } else if (state === "alert") {
    background = "#2a1215";
    shadow = ring(C.red, 1.5);
    color = C.red;
  }

  return (
    <div
      onPointerDown={press.onPointerDown}
      onClick={inert ? undefined : onClick}
      style={{
        height: 80,
        borderRadius: 14,
        background,
        boxShadow: shadow,
        color,
        opacity: state === "disabled" ? 0.35 : 1,
        transform: down ? "scale(.94)" : "none",
        transition: "transform .12s",
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        justifyContent: "center",
        gap: state === "busy" ? 8 : 5,
      }}
    >
      {state === "busy" ? (
        <span
          style={{
            width: 24,
            height: 24,
            borderRadius: "50%",
            border: `2.5px solid ${C.pressedRing}`,
            borderTopColor: C.accent,
            boxSizing: "border-box",
            animation: "spin .8s linear infinite",
          }}
        />
      ) : (
        <Icon name={icon} size={26} fill={state === "active"} />
      )}
      <span
        style={{
          fontSize: 12,
          fontWeight: state === "active" || state === "alert" ? 600 : 500,
          color: state === "busy" ? C.text3 : undefined,
        }}
      >
        {label}
      </span>
    </div>
  );
}
