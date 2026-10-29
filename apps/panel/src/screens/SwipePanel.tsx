import { useState } from "react";
import { Carousel } from "../kit/Carousel";
import { Icon } from "../kit/Icon";
import { Panel } from "../kit/Panel";
import { RowAction, SwipeRow } from "../kit/SwipeRow";
import { toast } from "../kit/toast";
import { C, FONT_MONO, caption, ring } from "../kit/tokens";

const SCENES = [
  { icon: "sports_esports", name: "Gameplay", sub: "Game · Cam" },
  { icon: "chat_bubble", name: "Just Chatting", sub: "Cam · Chat" },
  { icon: "schedule", name: "Starting Soon", sub: "Countdown" },
  { icon: "coffee", name: "Be Right Back", sub: "Lo-fi loop" },
];

const MESSAGES = [
  { user: "kairo", color: "#fb923c", text: "1v3 with 12 hp LMAO" },
  { user: "neonwolf", color: "#f97316", text: "what is that screen under your monitor??" },
  { user: "drift", color: "#60a5fa", text: "W streamer, the mixer is so clean" },
];

export function SwipePanel() {
  const [page, setPage] = useState(1);
  const [live, setLive] = useState(1);
  const [rests, setRests] = useState([0, 0, 0]);
  const [pinned, setPinned] = useState([false, false, false]);
  const [deleted, setDeleted] = useState([false, false, false]);

  const setAt = <T,>(list: T[], i: number, v: T) => list.map((x, j) => (j === i ? v : x));
  const close = (i: number) => setRests((r) => setAt(r, i, 0));

  const cards = SCENES.map((s, i) => (
    <div
      key={s.name}
      style={{
        position: "relative",
        width: "100%",
        height: "100%",
        boxSizing: "border-box",
        borderRadius: 14,
        background: C.control,
        boxShadow: i === live ? ring(C.text, 2) : ring(C.controlRing),
        padding: 18,
        display: "flex",
        flexDirection: "column",
        justifyContent: "space-between",
      }}
    >
      <Icon name={s.icon} size={30} color={C.accent} />
      <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
        <span style={{ fontSize: 20, fontWeight: 600 }}>{s.name}</span>
        <span style={{ fontFamily: FONT_MONO, fontSize: 11, color: C.text4 }}>{s.sub}</span>
      </div>
      {i === live && (
        <span
          style={{
            position: "absolute",
            top: 18,
            right: 18,
            display: "flex",
            alignItems: "center",
            gap: 6,
            height: 24,
            padding: "0 10px",
            borderRadius: 999,
            background: C.text,
            color: C.bg,
            fontSize: 11,
            fontWeight: 600,
          }}
        >
          <span style={{ width: 6, height: 6, borderRadius: 3, background: "#ef4444" }} />
          On air
        </span>
      )}
    </div>
  ));

  return (
    <Panel index="04" title="Swipe" badgeIcon="swipe" badge="page · reveal" gap={12}>
      <div style={{ display: "flex", justifyContent: "space-between", fontSize: 13, marginTop: 2 }}>
        <span style={{ fontWeight: 500 }}>
          Scenes <span style={caption}>· swipe, tap to go live</span>
        </span>
        <span style={{ fontFamily: FONT_MONO, color: C.text2 }}>
          {page + 1} / {SCENES.length}
        </span>
      </div>
      <Carousel
        cards={cards}
        index={page}
        onIndexChange={setPage}
        onSelect={(i) => {
          if (live !== i) toast(`${SCENES[i].name} is live`);
          setLive(i);
        }}
      />
      <div style={{ fontSize: 13, fontWeight: 500 }}>
        Chat <span style={caption}>· swipe row ← timeout / delete · → pin</span>
      </div>
      <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
        {MESSAGES.map((m, i) => (
          <SwipeRow
            key={m.user}
            rest={rests[i]}
            onRest={(x) => setRests((r) => setAt(r, i, x))}
            onGrab={() => setRests((r) => r.map((v, j) => (j === i ? v : 0)))}
            leftAction={
              <RowAction
                wide
                background={C.accent}
                color={C.accentInk}
                icon="push_pin"
                label={pinned[i] ? "Unpin" : "Pin"}
                onClick={() => {
                  toast(pinned[i] ? "Unpinned" : `Pinned ${m.user}`);
                  setPinned((p) => setAt(p, i, !p[i]));
                  close(i);
                }}
              />
            }
            rightActions={
              <>
                <RowAction
                  background={C.amber}
                  color="#1a1002"
                  icon="timer"
                  label="Timeout"
                  onClick={() => {
                    toast(`${m.user} timed out · 10 min`);
                    close(i);
                  }}
                />
                <RowAction
                  background={C.red}
                  color="#1f0606"
                  icon="delete"
                  label="Delete"
                  onClick={() => {
                    toast("Message deleted");
                    setDeleted((d) => setAt(d, i, true));
                    close(i);
                  }}
                />
              </>
            }
          >
            {pinned[i] && <Icon name="push_pin" size={16} color={C.accent} />}
            <span style={{ flex: "none", fontSize: 13, fontWeight: 600, color: m.color }}>{m.user}</span>
            <span
              style={{
                minWidth: 0,
                fontSize: 13,
                color: deleted[i] ? C.text5 : C.text2,
                overflow: "hidden",
                textOverflow: "ellipsis",
                whiteSpace: "nowrap",
              }}
            >
              {deleted[i] ? "message deleted" : m.text}
            </span>
          </SwipeRow>
        ))}
      </div>
    </Panel>
  );
}
