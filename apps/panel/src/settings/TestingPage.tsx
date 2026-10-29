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

/** Display sizes to try the panel UI at (CSS px). */
const SIZES = [
  { value: "1920x550", label: "1920 × 550 · bar (this project's panel)" },
  { value: "2560x720", label: "2560 × 720 · wide bar" },
  { value: "1920x480", label: "1920 × 480 · flat bar" },
  { value: "1920x1080", label: "1920 × 1080 · 16:9" },
  { value: "1280x800", label: "1280 × 800 · 16:10" },
  { value: "1024x600", label: "1024 × 600 · small" },
  { value: "800x480", label: "800 × 480 · tiny" },
  { value: "720x1280", label: "720 × 1280 · portrait" },
];

/** Testing mode: virtual panels in windows on the main monitor, fed by recorded touches. */
export function TestingPage({ view }: { view: ConfigView }) {
  const save = useSave();
  const t = view.effective.testing;
  const isSet = (k: string) => fileValue(view.file, `testing.${k}`) !== undefined;
  const virtuals = usePanelStatus().filter((s) => s.is_virtual);
  // A file path set in config.json shows as its own option.
  const replays = REPLAYS.some((r) => r.value === t.replay) ? REPLAYS : [...REPLAYS, { value: t.replay, label: t.replay }];
  const size = `${t.size[0]}x${t.size[1]}`;
  const sizes = SIZES.some((s) => s.value === size) ? SIZES : [...SIZES, { value: size, label: size.replace("x", " × ") }];

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
          label="Size"
          hint="Each virtual panel's size, to see how the panel UI fits other displays: one row on bars, a grid on 16:9 or portrait, pages with dots on small screens."
          isSet={isSet("size")}
          onReset={() => save("testing.size", null)}
        >
          <Select value={size} options={sizes} onChange={(v) => save("testing.size", v.split("x").map(Number))} />
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
