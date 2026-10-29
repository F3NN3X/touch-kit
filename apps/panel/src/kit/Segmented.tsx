import { C, ring } from "./tokens";
import { usePress } from "./usePress";

interface SegmentedProps {
  options: string[];
  value: number;
  onChange: (index: number) => void;
}

interface SegmentProps {
  label: string;
  selected: boolean;
  onClick: () => void;
}

function Segment({ label, selected, onClick }: SegmentProps) {
  const press = usePress();
  return (
    <div
      onPointerDown={press.onPointerDown}
      onClick={onClick}
      style={{
        borderRadius: 10,
        background: selected ? C.pressed : press.pressed ? C.panelRing : "transparent",
        color: selected ? C.text : C.text4,
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        fontSize: 13,
        fontWeight: 500,
        transition: "background .15s",
      }}
    >
      {label}
    </div>
  );
}

export function Segmented({ options, value, onChange }: SegmentedProps) {
  return (
    <div
      style={{
        height: 52,
        borderRadius: 14,
        background: C.well,
        boxShadow: ring(C.panelRing),
        padding: 4,
        boxSizing: "border-box",
        display: "grid",
        gridTemplateColumns: `repeat(${options.length}, minmax(0,1fr))`,
        gap: 4,
      }}
    >
      {options.map((label, i) => (
        <Segment key={label} label={label} selected={i === value} onClick={() => onChange(i)} />
      ))}
    </div>
  );
}
