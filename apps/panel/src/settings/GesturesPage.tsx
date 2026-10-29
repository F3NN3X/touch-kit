import { fileValue, type ConfigView } from "./api";
import { NumberField, Row, Section, useSave } from "./controls";

interface Threshold {
  key: string;
  label: string;
  hint: string;
  unit: string;
  step?: number;
}

// Defaults are Touch Kit spec §2; touch-core and the UI kit both use these values.
const GROUPS: { title: string; items: Threshold[] }[] = [
  {
    title: "Tap and hold",
    items: [
      { key: "slop_px", label: "Tap slop", hint: "Movement that still counts as a tap.", unit: "px" },
      { key: "long_press_ms", label: "Long press", hint: "Hold time for a long press and hold-to-confirm controls.", unit: "ms" },
      { key: "long_press_repeat_ms", label: "Long press repeat", hint: "Repeat while held after a long press; 0 = off.", unit: "ms" },
      { key: "double_tap_gap_ms", label: "Double tap gap", hint: "Longest pause between the two taps.", unit: "ms" },
      { key: "double_tap_slop_px", label: "Double tap distance", hint: "Furthest apart the two taps may be.", unit: "px" },
    ],
  },
  {
    title: "Swipe",
    items: [
      { key: "swipe_min_px", label: "Swipe distance", hint: "A stroke longer than this is a swipe at any speed.", unit: "px" },
      { key: "flick_min_px", label: "Flick distance", hint: "A shorter stroke still swipes if longer than this…", unit: "px" },
      { key: "flick_min_speed", label: "Flick speed", hint: "…and faster than this.", unit: "px/ms", step: 0.05 },
      { key: "swipe_axis_ratio", label: "Axis ratio", hint: "The main axis must be this many times the other (≥ 1).", unit: "×", step: 0.1 },
    ],
  },
  {
    title: "Multi-finger",
    items: [
      { key: "multi_finger_window_ms", label: "Finger window", hint: "Fingers landing within this time form one gesture.", unit: "ms" },
      { key: "pinch_min_scale", label: "Pinch start", hint: "Span change before two fingers become a pinch (0–1).", unit: "×", step: 0.01 },
      { key: "three_tap_max_ms", label: "Three-finger tap", hint: "Longest a three-finger tap may last.", unit: "ms" },
      { key: "lift_debounce_ms", label: "Lift debounce", hint: "A lift followed this soon by a touch nearby is one contact.", unit: "ms" },
      { key: "lift_debounce_px", label: "Lift debounce distance", hint: "…and this close.", unit: "px" },
    ],
  },
];

export function GesturesPage({ view }: { view: ConfigView }) {
  const save = useSave();
  return (
    <>
      {GROUPS.map((g) => (
        <Section key={g.title} title={g.title}>
          {g.items.map((t) => {
            const key = `gestures.${t.key}`;
            const def = view.defaults.gestures[t.key];
            return (
              <Row
                key={t.key}
                label={t.label}
                hint={`${t.hint} Default ${def} ${t.unit}.`}
                isSet={fileValue(view.file, key) !== undefined}
                onReset={() => save(key, null)}
              >
                <NumberField value={view.effective.gestures[t.key]} step={t.step} min={0} unit={t.unit} onCommit={(v) => save(key, v)} />
              </Row>
            );
          })}
        </Section>
      ))}
    </>
  );
}
