// Copy-to-clipboard icon button; the icon turns into a check for a moment.
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
    void navigator.clipboard.writeText(text).then(() => {
      setCopied(true);
      if (timer.current !== null) window.clearTimeout(timer.current);
      timer.current = window.setTimeout(() => setCopied(false), 1200);
    });
  }, [text]);

  return (
    <IconButton label={copied ? "Copied" : label} onClick={handleCopy} className={cn(copied && "text-ok", className)}>
      <span className="relative h-4 w-4">
        <span
          className={cn(
            "absolute inset-0 transition-[opacity,transform] duration-200 ease-out",
            copied ? "scale-50 opacity-0" : "scale-100 opacity-100",
          )}
        >
          <Icon name="content_copy" size={16} />
        </span>
        <span
          className={cn(
            "absolute inset-0 transition-[opacity,transform] duration-200 ease-spring",
            copied ? "scale-100 opacity-100" : "scale-50 opacity-0",
          )}
        >
          <Icon name="check" size={16} strokeWidth={2.25} />
        </span>
      </span>
    </IconButton>
  );
}
