import { fileValue, usePanelStatus, type ConfigView } from "./api";
import { NumberField, Row, Section, Select, Toggle, useSave } from "./controls";
import { touchText } from "./PanelsPage";

const REPLAYS = [
  { value: "all", label: "All captures, in turn" },
  { value: "tap", label: "Taps" },
  { value: "swipes", label: "Swipes" },
  { value: "two-finger", label: "Two-finger swipe and pinch" },
  { value: "hold", label: "Long presses" },
  { value: "none", label: "Nothing (use the mouse)" },
];

/** Testing mode: virtual panels in windows on the main monitor, fed by recorded touches. */
export function TestingPage({ view }: { view: ConfigView }) {
  const save = useSave();
  const t = view.effective.testing;
  const isSet = (k: string) => fileValue(view.file, `testing.${k}`) !== undefined;
  const virtuals = usePanelStatus().filter((s) => s.is_virtual);
  // A file path set in config.json shows as its own option.
  const replays = REPLAYS.some((r) => r.value === t.replay) ? REPLAYS : [...REPLAYS, { value: t.replay, label: t.replay }];

  return (
    <>
      <Section title="Virtual panels">
        <Row
          label="Testing mode"
          hint="Extra panels in normal windows on the main monitor, each playing recorded touches in a loop instead of a USB device. The real panels keep running. Rebuilds the panel windows."
          isSet={isSet("enabled")}
          onReset={() => save("testing.enabled", null)}
        >
          <Toggle on={t.enabled} onChange={(v) => save("testing.enabled", v)} />
        </Row>
        <Row
          label="Count"
          hint="How many virtual panels (1–4). They stack down the main monitor at the panel's size."
          isSet={isSet("virtual_panels")}
          onReset={() => save("testing.virtual_panels", null)}
        >
          <NumberField value={t.virtual_panels} min={1} max={4} onCommit={(v) => save("testing.virtual_panels", v)} />
        </Row>
        <Row
          label="Plays"
          hint="Touches recorded on the real panel. A file from touchprobe record can go in config.json as testing.replay."
          isSet={isSet("replay")}
          onReset={() => save("testing.replay", null)}
        >
          <Select value={t.replay} options={replays} onChange={(v) => save("testing.replay", v)} />
        </Row>
      </Section>
      {t.enabled && (
        <Section title="Running">
          {virtuals.length === 0 ? (
            <p className="py-3 text-sm text-zinc-500">None running · see the issues above.</p>
          ) : (
            virtuals.map((s) => {
              const touch = touchText(s.touch);
              return (
                <div key={s.index} className="flex items-center justify-between border-b border-white/5 py-3 text-sm last:border-0">
                  <span className="text-zinc-100">{s.name}</span>
                  <span className={touch.tone}>{touch.text}</span>
                </div>
              );
            })
          )}
        </Section>
      )}
    </>
  );
}
