// Copy-to-clipboard icon button; the icon becomes a check for a moment.
import { useCallback, useEffect, useRef, useState } from "react";
import { Icon } from "@/components/ui/Icon";
import { IconButton } from "@/components/ui/Button";
import { cn } from "@/lib/utils";

export function CopyButton({ text, label = "Copy", className }: { text: string; label?: string; className?: string }) {
  const [copied, setCopied] = useState(false);
  const timer = useRef<number | null>(null);

  useEffect(() => {
    return () => {
      if (timer.current !== null) window.clearTimeout(timer.current);
    };
  }, []);

  const handleCopy = useCallback(() => {
    navigator.clipboard
      .writeText(text)
      .then(() => {
        setCopied(true);
        if (timer.current !== null) window.clearTimeout(timer.current);
        timer.current = window.setTimeout(() => setCopied(false), 1200);
      })
      // No tick when nothing was copied.
      .catch((error: unknown) => console.error("Copy failed:", error));
  }, [text]);

  return (
    <IconButton label={copied ? "Copied" : label} onClick={handleCopy} className={cn(copied && "text-ink", className)}>
      <Icon name={copied ? "check" : "content_copy"} size={14} strokeWidth={copied ? 2.25 : 1.75} />
    </IconButton>
  );
}
