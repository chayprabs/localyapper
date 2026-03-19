# React Frontend Context

## Structure
- App.tsx — Root with routing/page switching
- components/ — UI components (PascalCase directories)
- components/ui/ — shadcn/ui primitives (Button, Card, Input, etc.)
- components/overlay/ — Overlay pill components (Waveform, CountdownTimer, YappingEmoji)
- components/wizard/ — First-launch wizard screens (10 steps)
- components/dashboard/ — Dashboard page components
- components/history/ — History page components
- components/dictionary/ — Dictionary + Training page components
- components/hotkeys/ — Hotkey remapping components
- components/models/ — Model settings components
- hooks/ — Custom React hooks (useInvoke, useSettings, useOverlayState, etc.)
- stores/ — Jotai atoms (appStore.ts for all global state)
- lib/commands/ — Typed Tauri command wrappers (one file per domain)
- types/ — TypeScript definitions
- types/commands.ts — IPC types (MUST match Rust structs exactly)

## Rules
- Functional components only, React 19 features welcome (use, Actions)
- TypeScript strict mode — no `any`, no `as` without justifying comment
- Named exports only — no default exports
- Path aliases: @/ for src/, @components/, @hooks/, @stores/, @lib/
- Tailwind utility classes only — NO separate CSS files
- shadcn/ui components for all UI primitives — never build custom buttons/inputs/cards
- Jotai for global state, local useState for component-only state
- No business logic in components — extract to hooks or stores

## Tauri command wrapper pattern
```typescript
import { invoke } from '@tauri-apps/api/core';
import type { MyRequest, MyResponse } from '@/types/commands';

export async function myCommand(params: MyRequest): Promise<MyResponse> {
  return invoke<MyResponse>('my_command', params);
}
```
All invoke() calls go through lib/commands/ wrappers — components never call invoke() directly.

## Design system compliance
- See DESIGN_SYSTEM.md in repo root for ALL design tokens
- Window background: bg-window-bg (#f9f9f9)
- Sidebar background: bg-sidebar-bg (#eeeeee)
- Card: bg-white rounded-[10px] border border-card-border p-4
- Primary button: bg-primary text-white rounded-lg h-9 text-[13px] font-medium
- Primary accent: #0058bc
- Text primary: text-black/85
- Text secondary: text-black/50
- Text tertiary: text-black/[0.26]
- Section labels: text-[10px] uppercase tracking-[0.06em] text-black/[0.26]
- Font: font-['Inter'] — weights 400 and 600 only
- Light mode only. No dark mode.

## 26 screens total
- 10 wizard screens (onboarding flow)
- 5 overlay states (listening, stopping-soon, processing, long-recording, transcribed)
- 6 main app pages (Dashboard, History, Dictionary/Corrections, Dictionary/Training, Hotkeys, Models)
- 3 empty states (Dashboard empty, History empty, Dictionary empty)
- 2 training completion screens

## Overlay pill specs
- Width: 280px, border-radius: 999px (full pill)
- Background: rgba(255,255,255,0.95), border: 1px solid rgba(0,0,0,0.10)
- Shadow: 0 4px 24px rgba(0,0,0,0.15)
- Always on top, no decorations, transparent, skip taskbar
- Listens to Tauri events for state transitions
