// Icons: short names mapped to tree-shakable lucide-react SVGs, so only the
// icons that are drawn ship in the bundle.
import type { CSSProperties } from "react";
import {
  AlertCircle,
  AlertTriangle,
  ArrowLeft,
  AudioLines,
  Check,
  CheckCircle2,
  ChevronDown,
  Copy,
  Download,
  ExternalLink,
  History,
  House,
  Keyboard,
  Loader2,
  Mic,
  MicOff,
  Minus,
  Pause,
  Search,
  SlidersHorizontal,
  Square,
  Trash2,
  Upload,
  X,
  type LucideIcon,
} from "lucide-react";

const REGISTRY = {
  mic: Mic,
  mic_off: MicOff,
  delete: Trash2,
  download: Download,
  export: Upload,
  pause: Pause,
  arrow_back: ArrowLeft,
  progress_activity: Loader2,
  check_circle: CheckCircle2,
  check: Check,
  content_copy: Copy,
  error: AlertCircle,
  warning: AlertTriangle,
  keyboard: Keyboard,
  history: History,
  open_in_new: ExternalLink,
  expand_more: ChevronDown,
  voice: AudioLines,
  home: House,
  settings: SlidersHorizontal,
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
