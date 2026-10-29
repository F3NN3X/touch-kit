import { fileValue, type ConfigView } from "./api";
import { NumberField, Row, Section, Toggle, useSave } from "./controls";

export function GeneralPage({ view }: { view: ConfigView }) {
  const save = useSave();
  const e = view.effective;
  const isSet = (k: string) => fileValue(view.file, k) !== undefined;
  const toggle = (key: "dev_mode" | "debug" | "touch_dots", label: string, hint: string) => (
    <Row label={label} hint={hint} isSet={isSet(key)} onReset={() => save(key, null)}>
      <Toggle on={e[key]} onChange={(v) => save(key, v)} />
    </Row>
  );
  return (
    <>
      <Section title="Feedback">
        <Row
          label="Prediction"
          hint="How far ahead feedback draws a moving finger, to hide display latency. Too high overshoots when the finger stops."
          isSet={isSet("prediction_ms")}
          onReset={() => save("prediction_ms", null)}
        >
          <NumberField value={e.prediction_ms} min={0} max={100} unit="ms" onCommit={(v) => save("prediction_ms", v)} />
        </Row>
        {toggle("touch_dots", "Touch dots", "Rings under every finger, for demos and recordings.")}
        {toggle("debug", "Debug strip at start", "Latency and thresholds along the panel's bottom edge; a three-finger tap toggles it.")}
      </Section>
      <Section title="Development">
        {toggle("dev_mode", "Dev mode", "The real mouse reaches the panels and devtools open. Rebuilds the panel windows.")}
      </Section>
    </>
  );
}
