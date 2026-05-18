// Reusable copy-to-clipboard button with check animation feedback
import { useState, useCallback, useRef } from "react";
import { Icon } from "@/components/ui/Icon";

interface CopyButtonProps {
  /** Text content to write to clipboard on click. */
  text: string;
  /** "icon" shows Material Symbols copy icon; "text" shows "Copy" label. */
  variant?: "icon" | "text";
  /** Called 500ms after copy — used by overlay to dismiss after copying. */
  onAfterCopy?: () => void;
  className?: string;
  iconSize?: number;
}

export function CopyButton({
  text,
  variant = "icon",
  onAfterCopy,
  className,
  iconSize = 18,
}: CopyButtonProps) {
  const [copied, setCopied] = useState(false);
  const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const handleCopy = useCallback(async () => {
    if (copied) return;
    await navigator.clipboard.writeText(text);
    setCopied(true);

    if (timeoutRef.current) clearTimeout(timeoutRef.current);

    if (onAfterCopy) {
      timeoutRef.current = setTimeout(() => onAfterCopy(), 500);
    } else {
      timeoutRef.current = setTimeout(() => setCopied(false), 1000);
    }
  }, [text, onAfterCopy, copied]);

  if (variant === "text") {
    return (
      <button onClick={handleCopy} className={className}>
        <span
          className="inline-block transition-opacity duration-150"
          style={{ opacity: copied ? 0 : 1, position: copied ? "absolute" : "relative" }}
        >
          Copy
        </span>
        <span
          className="inline-block transition-opacity duration-150"
          style={{ opacity: copied ? 1 : 0, position: copied ? "relative" : "absolute" }}
        >
          &#x2713;
        </span>
      </button>
    );
  }

  return (
    <button onClick={handleCopy} className={className}>
      <span className="relative inline-flex items-center justify-center" style={{ width: iconSize, height: iconSize }}>
        <span
          className="absolute inset-0 flex items-center justify-center transition-opacity duration-150"
          style={{ opacity: copied ? 0 : 1 }}
        >
          <Icon name="content_copy" size={iconSize} />
        </span>
        <span
          className="absolute inset-0 flex items-center justify-center transition-opacity duration-150"
          style={{ opacity: copied ? 1 : 0 }}
        >
          <Icon name="check" size={iconSize} />
        </span>
      </span>
    </button>
  );
}
