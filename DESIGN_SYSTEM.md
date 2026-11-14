# LocalYapper Design System

## Idea

Quiet glass. One column of content floats on a soft field of colour; panels
are translucent, edges catch light, and almost everything is ink on glass.
Colour is spent in one place at a time: the accent marks what is on, live or
being edited. Light and dark themes are equal citizens.

## Structure

- **Title bar** (56 px, draggable): logo, the page tabs in a glass track,
  window controls. A lifted pill slides to the active tab. Below 768 px wide
  only the active tab keeps its label.
- **Pages**: Home, History, Dictation, Voice, General. Each is a centred
  column, 680 px at most, that scrolls under the title bar with a soft fade.
- **Wizard**: one glass card, one step at a time, progress dots on top.
- **Overlay**: a dark glass capsule. Its window is transparent, so it uses
  the same dark colours in both themes and reads on any desktop.

## Tokens

Defined in `src/index.css` as RGB channels and exposed to Tailwind in
`tailwind.config.ts`, so opacity modifiers work in both themes
(`text-ink/50`, `bg-accent/10`).

| Token | Light | Dark | Use |
|-------|-------|------|-----|
| `ink` | 18 18 24 | 244 244 250 | Text, icons, fills at low opacity |
| `canvas` | 243 244 248 | 11 11 15 | Window background |
| `accent` | 84 82 232 | 150 148 255 | On, live, editing, progress |
| `ok` | 21 128 61 | 74 222 128 | Ready, detected |
| `warn` | 161 98 7 | 251 191 36 | Paused, needs attention |
| `danger` | 200 30 30 | 248 113 113 | Errors, destructive actions |

Never hard-code a colour in a component. Text opacity steps: 100 (titles),
85 to 90 (body), 45 to 55 (secondary), 35 (placeholders).

## Surfaces

- `.glass`: translucent fill, 28 px backdrop blur, a bright 1 px edge, an
  inner top highlight and a soft drop shadow. Panels, the tab track, the
  hero disc, the search field.
- `.raised`: the opaque chip lifted off a glass track. The active tab, the
  chosen segment, key caps.
- Inside a panel, controls sit on `bg-ink/[0.06]` (hover `0.1`). Rows are
  separated by `divide-ink/[0.06]` hairlines; no borders around controls.
- The ambient background is three radial gradients on `body::before` that
  drift slowly. It uses no blur filter.

## Shape and type

- Radii: panels 20 px, wizard card 28 px, inner wells 16 px, key caps 7 px.
  Buttons, selects, switches, chips and fields are fully rounded.
- System font stack; no web fonts are loaded. Sizes: 28 hero, 22 page
  title, 15 dictated text, 13 body and controls, 12 hints and section
  labels, 11 chips. Titles use negative tracking; numbers use
  `tabular-nums`.
- Section labels are sentence case, 12 px, 45% ink. No uppercase labels.
- Icons: lucide through `components/ui/Icon.tsx`, stroke 1.75, 14 to 16 px
  in controls.

## Components

`components/ui`: `Button` (primary, soft, ghost, danger), `IconButton`,
`Switch`, `Keys`, `CopyButton`, `Icon`.
`components/settings/controls.tsx`: `Page`, `PageHeader`, `Panel`,
`Section`, `Row`, `Notice`, `Chip`, `Select`, `Segmented`, and the setting
controls (`ToggleSetting`, `SelectSetting`, `SegmentedSetting`,
`NumberSetting`) that load, save and show the backend's error for one key.

- One primary (ink) button per view at most.
- Destructive actions confirm in place: the control turns into
  Cancel / danger. No modal dialogs.
- Use `Segmented` for two or three short options, `Select` for more.

## Motion

- Pages and wizard steps rise in, their blocks staggered by 50 ms
  (`.stagger`).
- The tab pill slides (300 ms, ease-out); switches use a slight spring.
- Home: two rings pulse from the disc while dictation is available;
  numbers count up.
- Feedback appears with `animate-fade-in` or `animate-rise-in`.
- Everything stops under `prefers-reduced-motion`.

## Voice

Plain and short. Say what happens, where the data stays, and what to do
next. Sentence case everywhere.
