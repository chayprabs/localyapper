// Animated 5-bar audio waveform visualization
interface WaveformProps {
  color: "blue" | "red";
}

// Bar heights for the 5-bar waveform pattern
const baseHeights = [8, 16, 22, 16, 8];
const animations = [
  "animate-waveform-1",
  "animate-waveform-2",
  "animate-waveform-3",
  "animate-waveform-4",
  "animate-waveform-5",
] as const;

export function Waveform({ color }: WaveformProps) {
  const barColor = color === "blue" ? "bg-primary" : "bg-destructive";

  return (
    <div className="flex items-center gap-[4px]">
      {baseHeights.map((h, i) => (
        <div
          key={i}
          className={`w-[3px] rounded-full ${barColor} ${animations[i]}`}
          style={{ height: `${h}px` }}
        />
      ))}
    </div>
  );
}
