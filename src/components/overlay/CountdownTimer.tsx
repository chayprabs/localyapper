interface CountdownTimerProps {
  mode: "elapsed" | "countdown";
  seconds: number;
}

export function CountdownTimer({ mode, seconds }: CountdownTimerProps) {
  if (mode === "countdown") {
    return (
      <span className="text-[17px] font-semibold text-[#FF3B30]">
        {Math.ceil(seconds)}
      </span>
    );
  }

  return (
    <span className="text-[17px] font-bold text-[#1C1C1E]">
      {seconds.toFixed(1)}s
    </span>
  );
}
