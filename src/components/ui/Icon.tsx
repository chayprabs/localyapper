// Icon component — drop-in replacement for the Material Symbols font.
//
// Each WebView used to load a 3.9MB woff2 just to render ~20 glyphs. This
// maps the same Material-style names to tree-shakable lucide-react SVGs
// so only the icons we actually render ship in the bundle.
import type { CSSProperties } from "react";
import {
  AlertCircle,
  AlertTriangle,
  ArrowLeft,
  AudioWaveform,
  Check,
  CheckCircle2,
  ChevronDown,
  ChevronsLeft,
  ChevronsRight,
  Cloud,
  CloudDownload,
  CloudOff,
  Copy,
  Download,
  ExternalLink,
  FileText,
  HelpCircle,
  History,
  Keyboard,
  LayoutDashboard,
  Loader2,
  Mic,
  MicOff,
  Pause,
  RefreshCw,
  Trash2,
  type LucideIcon,
} from "lucide-react";

export type IconName =
  | "mic"
  | "mic_off"
  | "delete"
  | "description"
  | "download"
  | "cloud_download"
  | "cloud_off"
  | "cloud"
  | "pause"
  | "arrow_back"
  | "progress_activity"
  | "check_circle"
  | "check"
  | "content_copy"
  | "error"
  | "warning"
  | "keyboard"
  | "keyboard_double_arrow_right"
  | "keyboard_double_arrow_left"
  | "history"
  | "open_in_new"
  | "expand_more"
  | "graphic_eq"
  | "dashboard"
  | "sync"
  | "help";

const REGISTRY: Record<IconName, LucideIcon> = {
  mic: Mic,
  mic_off: MicOff,
  delete: Trash2,
  description: FileText,
  download: Download,
  cloud_download: CloudDownload,
  cloud_off: CloudOff,
  cloud: Cloud,
  pause: Pause,
  arrow_back: ArrowLeft,
  progress_activity: Loader2,
  check_circle: CheckCircle2,
  check: Check,
  content_copy: Copy,
  error: AlertCircle,
  warning: AlertTriangle,
  keyboard: Keyboard,
  keyboard_double_arrow_right: ChevronsRight,
  keyboard_double_arrow_left: ChevronsLeft,
  history: History,
  open_in_new: ExternalLink,
  expand_more: ChevronDown,
  graphic_eq: AudioWaveform,
  dashboard: LayoutDashboard,
  sync: RefreshCw,
  help: HelpCircle,
};

interface IconProps {
  name: IconName;
  /** Pixel size — matches the old text-[Npx] Material Symbols pattern. */
  size?: number;
  /** Extra classes to compose alongside any layout/colour utilities. */
  className?: string;
  style?: CSSProperties;
  /** SVG stroke weight. Lucide defaults to 2; lighter feels closer to the
   *  outlined-400 Material Symbols look. */
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
