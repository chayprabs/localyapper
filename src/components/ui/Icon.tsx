// Icons: short names mapped to tree-shakable lucide-react SVGs, so only the
// icons that are drawn ship in the bundle.
import type { CSSProperties } from "react";
import {
  AlertTriangle,
  ArrowLeft,
  Check,
  ChevronDown,
  Copy,
  Loader2,
  Minus,
  Search,
  Square,
  Trash2,
  X,
  type LucideIcon,
} from "lucide-react";

const REGISTRY = {
  delete: Trash2,
  arrow_back: ArrowLeft,
  progress_activity: Loader2,
  check: Check,
  content_copy: Copy,
  warning: AlertTriangle,
  expand_more: ChevronDown,
  search: Search,
  minimize: Minus,
  maximize: Square,
  close: X,
} satisfies Record<string, LucideIcon>;

export type IconName = keyof typeof REGISTRY;

interface IconProps {
  name: IconName;
  /** Pixel size. */
  size?: number;
  /** Extra classes to compose alongside any layout/colour utilities. */
  className?: string;
  style?: CSSProperties;
  /** SVG stroke weight. */
  strokeWidth?: number;
}

export function Icon({
  name,
  size = 20,
  className,
  style,
  strokeWidth = 1.75,
}: IconProps) {
  const Component = REGISTRY[name];
  return (
    <Component
      size={size}
      strokeWidth={strokeWidth}
      className={className}
      style={style}
      aria-hidden="true"
    />
  );
}
