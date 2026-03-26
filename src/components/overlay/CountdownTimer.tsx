// Timer display -- elapsed seconds or countdown for recording limits
interface CountdownTimerProps {
  /** "elapsed" shows seconds.tenths (e.g. "3.2s"); "countdown" shows integer ceiling (e.g. "12"). */
  mode: "elapsed" | "countdown";
  /** Current time value — interpreted differently based on mode. */
  seconds: number;
}

export function CountdownTimer({ mode, seconds }: CountdownTimerProps) {
  if (mode === "countdown") {
    return (
      <span className="text-[17px] font-semibold text-destructive">
        {Math.ceil(seconds)}
      </span>
    );
  }

  return (
    <span className="text-[17px] font-bold text-black/85">
      {seconds.toFixed(1)}s
    </span>
  );
}
