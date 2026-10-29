import { useState, type PointerEvent } from "react";
import { track } from "./drag";

/** Press feedback as state: `:active` never fires for the bridge's synthetic pointer events. */
export function usePress(inert = false) {
  const [pressed, setPressed] = useState(false);
  const onPointerDown = (e: PointerEvent<HTMLElement>) => {
    if (inert) return;
    setPressed(true);
    track(e, undefined, () => setPressed(false));
  };
  return { pressed, onPointerDown };
}
