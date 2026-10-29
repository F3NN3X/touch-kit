import { useState, type ReactNode } from "react";
import { HoldButton } from "../kit/HoldButton";
import { HoldToggle } from "../kit/HoldToggle";
import { Panel } from "../kit/Panel";
import { Segmented } from "../kit/Segmented";
import { Switch } from "../kit/Switch";
import { Tile } from "../kit/Tile";
import { caption } from "../kit/tokens";

interface CellProps {
  caption: string;
  children: ReactNode;
}

function Cell({ caption: text, children }: CellProps) {
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
      {children}
      <span style={{ ...caption, textAlign: "center" }}>{text}</span>
    </div>
  );
}

export function ButtonsPanel() {
  const [seg, setSeg] = useState(0);
  const [rec, setRec] = useState(true);
  const [muted, setMuted] = useState(false);
  const [live, setLive] = useState(true);
  return (
    <Panel index="01" title="Buttons" badgeIcon="touch_app" badge="tap · hold">
      <div style={{ display: "grid", gridTemplateColumns: "repeat(2, minmax(0,1fr))", gap: 10 }}>
        <Cell caption={muted ? "hold · unmute" : "hold · mute"}>
          <HoldToggle
            on={muted}
            onChange={setMuted}
            off={{ icon: "mic", label: "Mic", toast: "Mic on" }}
            onFace={{ icon: "mic_off", label: "Muted", toast: "Mic muted" }}
          />
        </Cell>
        <Cell caption={live ? "active · tap" : "tap · go live"}>
          <Tile icon="sensors" label="Live" state={live ? "active" : "rest"} onClick={() => setLive(!live)} />
        </Cell>
        <Cell caption="disabled · 35%">
          <Tile icon="replay" label="Replay" state="disabled" />
        </Cell>
        <Cell caption="busy">
          <Tile icon="" label="Saving" state="busy" />
        </Cell>
      </div>
      <Segmented options={["Scenes", "Chat", "Mixer"]} value={seg} onChange={setSeg} />
      <div style={{ display: "grid", gridTemplateColumns: "minmax(0,1fr) minmax(0,1fr)", gap: 10 }}>
        <Switch label="Record" hint={rec ? "on" : "off"} on={rec} onChange={setRec} />
        <HoldButton icon="replay" label="Hold · save replay" doneLabel="Saved" doneToast="Replay saved · last 30 s" />
      </div>
    </Panel>
  );
}
