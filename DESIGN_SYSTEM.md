# LocalYapper Design System

## Philosophy
Apple macOS Human Interface Guidelines. Light mode only.
Every decision references macOS Ventura System Settings as the gold standard.
Nothing that Apple would never put there.

## No-Line Rule
Major section boundaries (sidebar <-> content) use background color shifts only — no visible border lines.
Cards still use 1px solid rgba(0,0,0,0.07) — this rule does NOT apply to cards.
Boundaries defined strictly through background color shifts.

## Glass & Gradient Rule
Floating elements (overlay pill, modals, tooltips) use glassmorphism:
- `background: rgba(255,255,255,0.80)`
- `backdrop-filter: blur(20px)`
- `-webkit-backdrop-filter: blur(20px)`

Subtle gradient on primary buttons (top-to-bottom, slight lightening).

## Elevation & Depth (Tonal Layering)
- **Level 0 (Base):** `#f9f9f9` — window background
- **Level 1 (Cards):** `#FFFFFF` — card surfaces, inputs
- **Level 2 (Floating):** glassmorphism (80% white + 20px blur)
- **Whisper Shadow:** `0 8px 32px rgba(0,0,0,0.04)` — subtle elevation
- **Ghost Border:** outline-variant at 15% opacity — very subtle edge definition

## Colors
| Token | Value | Usage |
|-------|-------|-------|
| window-bg | #f9f9f9 | App window background |
| sidebar-bg | #eeeeee | Sidebar background |
| card-bg | #FFFFFF | All card surfaces |
| card-border | rgba(0,0,0,0.07) | Card borders |
| card-radius | 10px | Card border radius |
| primary | #0058bc | Buttons, active states, accents |
| primary-tint | rgba(0,88,188,0.12) | Active sidebar item bg |
| success | #006b19 | Running status, success states |
| destructive | #ba1a1a | Delete, error, warning accents |
| text-primary | rgba(0,0,0,0.85) | All primary text (opacity-based) |
| text-secondary | rgba(0,0,0,0.50) | Descriptions, subtitles |
| text-tertiary | rgba(0,0,0,0.26) | Hints, captions, timestamps |
| text-label | rgba(0,0,0,0.26) | Uppercase section labels |
| separator | rgba(0,0,0,0.08) | Dividers inside cards |
| overlay-bg | rgba(255,255,255,0.80) | Floating overlay pill (glassmorphism) |
| overlay-border | rgba(0,0,0,0.10) | Overlay pill border |

## Typography
| Role | Size | Weight | Usage |
|------|------|--------|-------|
| Large Title | 26px | 600 | Page titles |
| Title 2 | 17px | 600 | Card titles, hero numbers |
| Headline | 13px | 600 | Row labels, action names |
| Body | 13px | 400 | All regular text |
| Callout | 12px | 400 | Descriptions under headlines |
| Caption | 11px | 400 | Timestamps, counts, hints |
| Label | 10px | 500 | UPPERCASE section headers (letter-spacing 0.06em) |
| Stat | 28px | 600 | Dashboard stat numbers |
| Font | SF Pro / Inter | — | System font stack, two weights only: 400 + 600 |

## Components

### Cards
- Background: #FFFFFF
- Border: 1px solid rgba(0,0,0,0.07)
- Border-radius: 10px
- Padding: 16px
- Shadow: none (flat)

### Buttons — Primary
- Background: linear-gradient(180deg, #0062d0 0%, #0058bc 100%)
- Hover: linear-gradient(180deg, #0058bc 0%, #004ea8 100%)
- Text: white, 13px weight 500
- Height: 36px
- Border-radius: 8px

### Buttons — Secondary
- Background: #FFFFFF
- Border: 1px solid rgba(0,0,0,0.15)
- Text: rgba(0,0,0,0.85), 13px
- Height: 36px
- Border-radius: 8px

### Buttons — Destructive text
- Background: none
- Text: #ba1a1a, 13px
- No border

### Sidebar
- Width: 220px
- Background: #eeeeee
- Active item: rgba(0,88,188,0.12) bg + #0058bc text + icon
- Item height: 36px
- Item border-radius: 6px
- Section labels: 10px uppercase rgba(0,0,0,0.26)
- No border between sidebar and content (No-Line Rule)

### Overlay pill
- Width: 280px
- Height: 52px (listening/processing) / 72px (transcribed/long recording)
- Border-radius: 999px
- Background: rgba(255,255,255,0.80)
- Backdrop-filter: blur(20px)
- -webkit-backdrop-filter: blur(20px)
- Border: 1px solid rgba(0,0,0,0.10)
- Shadow: 0 8px 32px rgba(0,0,0,0.08)
- Padding: 12px 16px

### Wizard modal
- Width: 480px
- Background: #FFFFFF
- Border-radius: 12px
- Padding: 28px
- Shadow: 0 8px 40px rgba(0,0,0,0.15)
- Background behind modal: flat #E8E8E8
