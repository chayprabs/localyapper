# LocalYapper Design System

## Philosophy
Apple macOS Human Interface Guidelines. Light mode only.
Every decision references macOS Ventura System Settings as the gold standard.
Nothing that Apple would never put there.

## Colors
| Token | Value | Usage |
|-------|-------|-------|
| window-bg | #EDEDED | App window background |
| sidebar-bg | #EBEBEB | Sidebar background |
| card-bg | #FFFFFF | All card surfaces |
| card-border | rgba(0,0,0,0.07) | Card borders |
| card-radius | 10px | Card border radius |
| primary | #007AFF | Buttons, active states, accents |
| primary-tint | rgba(0,122,255,0.12) | Active sidebar item bg |
| success | #28CD41 | Running status, success states |
| destructive | #FF3B30 | Delete, error, warning accents |
| text-primary | #1C1C1E | All primary text |
| text-secondary | rgba(0,0,0,0.50) | Descriptions, subtitles |
| text-tertiary | rgba(0,0,0,0.35) | Hints, captions, timestamps |
| text-label | rgba(0,0,0,0.40) | Uppercase section labels |
| separator | rgba(0,0,0,0.08) | Dividers inside cards |
| overlay-bg | rgba(255,255,255,0.95) | Floating overlay pill |
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
- Background: #007AFF
- Text: white, 13px weight 500
- Height: 36px
- Border-radius: 8px

### Buttons — Secondary
- Background: #FFFFFF
- Border: 1px solid rgba(0,0,0,0.15)
- Text: #1C1C1E, 13px
- Height: 36px
- Border-radius: 8px

### Buttons — Destructive text
- Background: none
- Text: #FF3B30, 13px
- No border

### Sidebar
- Width: 220px
- Background: #EBEBEB
- Active item: rgba(0,122,255,0.12) bg + #007AFF text + icon
- Item height: 36px
- Item border-radius: 6px
- Section labels: 10px uppercase rgba(0,0,0,0.40)

### Overlay pill
- Width: 280px
- Height: 52px (listening/processing) / 72px (transcribed/long recording)
- Border-radius: 999px
- Background: rgba(255,255,255,0.95)
- Border: 1px solid rgba(0,0,0,0.10)
- Shadow: 0 4px 24px rgba(0,0,0,0.15)
- Padding: 12px 16px

### Wizard modal
- Width: 480px
- Background: #FFFFFF
- Border-radius: 12px
- Padding: 28px
- Shadow: 0 8px 40px rgba(0,0,0,0.15)
- Background behind modal: flat #E8E8E8
