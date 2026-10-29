import { useState } from "react";
import { Knob } from "../kit/Knob";
import { LedFader } from "../kit/LedFader";
import { Panel } from "../kit/Panel";
import { SteppedSlider } from "../kit/SteppedSlider";
import { C, FONT_MONO, caption } from "../kit/tokens";
import { VerticalFader } from "../kit/VerticalFader";

const QUALITY = ["480p", "720p", "900p", "1080p", "1440p"];
const CHANNELS: [string, string][] = [
  ["Game", C.accent],
  ["Voice", "#7D95FF"],
  ["Music", "#f472b6"],
];

const readout = { fontFamily: FONT_MONO, color: C.text2 };

export function SlidersPanel() {
  const [master, setMaster] = useState(68);
  const [quality, setQuality] = useState(3);
  const [levels, setLevels] = useState([72, 54, 34]);
  const [knob, setKnob] = useState(0.55);
  const db = Math.round(-24 + knob * 36);

  return (
    <Panel index="03" title="Slide" badgeIcon="tune" badge="drag · rotate" gap={16}>
      <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
        <div style={{ display: "flex", justifyContent: "space-between", fontSize: 13 }}>
          <span style={{ fontWeight: 500 }}>Master</span>
          <span style={readout}>{master}%</span>
        </div>
        <LedFader value={master} onChange={setMaster} />
      </div>
      <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
        <div style={{ display: "flex", justifyContent: "space-between", fontSize: 13 }}>
          <span style={{ fontWeight: 500 }}>
            Quality <span style={caption}>· snaps</span>
          </span>
          <span style={readout}>{QUALITY[quality]}</span>
        </div>
        <SteppedSlider stops={QUALITY} value={quality} onChange={setQuality} />
      </div>
      <div style={{ flex: 1, minHeight: 0, display: "flex", gap: 14, alignItems: "stretch" }}>
        {CHANNELS.map(([name, color], i) => (
          <VerticalFader
            key={name}
            name={name}
            color={color}
            value={levels[i]}
            onChange={(v) => setLevels((l) => l.map((x, j) => (j === i ? v : x)))}
          />
        ))}
        <div style={{ flex: 1, display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", gap: 6 }}>
          <Knob value={knob} onChange={setKnob} />
          <span style={{ fontSize: 12, fontWeight: 500 }}>
            Gain <span style={caption}>· drag ↕</span>
          </span>
          <span style={{ ...caption, marginTop: -4 }}>
            {db > 0 ? "+" : ""}
            {db} dB
          </span>
        </div>
      </div>
    </Panel>
  );
}
