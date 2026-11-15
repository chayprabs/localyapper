# LocalYapper Design System

## Idea

A tool, not a showcase. Ink on a flat canvas, hairlines instead of boxes,
labels instead of explanations. If an element does not carry information or
an action, it is not there.

## Rules

- **No decoration.** No gradients, shadows, blur, glass, illustrations,
  icon badges or hero graphics.
- **One colour.** Everything is `ink` at some opacity on `canvas`. `signal`
  (red) is reserved for recording, errors and destructive actions.
- **Few words.** A row is a label and a control. Explanations go in the
  `hint` prop, which is a tooltip. No page titles (the tab says where you
  are), no subtitles, no marketing copy.
- **No cards.** Sections are a small label over a hairline; rows are
  separated by hairlines and sit flush with the column edge.
- **Mono for data.** Times, numbers, sizes, paths and keys use the
  monospace stack; everything else uses the system font.
- **Motion only for state.** The tab line slides, level bars follow the
  microphone, new content fades in (140 ms). Nothing loops for decoration.

## Structure

- **Title bar** (40 px, draggable, hairline below): logo, text tabs,
  window controls. A 1 px line slides under the active tab.
- **Pages**: History (the main page: record key, status, word counts,
  search, transcript list), Dictation, Voice, General. One centred column,
  620 px at most.
- **Wizard**: a left-aligned column with a step counter; one title, one
  sentence, the control, the buttons.
- **Overlay**: a flat near-black capsule (its window is transparent, so it
  is the same in both themes). Red bars while recording.

## Tokens

Defined in `src/index.css` as RGB channels, exposed to Tailwind in
`tailwind.config.ts` so opacity modifiers work (`text-ink/45`).

| Token | Light | Dark |
|-------|-------|------|
| `ink` | 20 20 19 | 236 236 232 |
| `canvas` | 252 252 250 | 18 18 17 |
| `signal` | 204 38 28 | 255 99 88 |

Ink opacity: 90 text, 45 to 55 secondary, 35 to 40 tertiary and mono data,
20 control borders, 15 section rules, 8 row rules.

## Shape and type

- Radius: 6 px controls, 4 px key caps. Only the switch and the overlay are
  round.
- Sizes: 13 body and labels, 12 section labels and secondary, 11.5 mono
  data, 22 wizard titles. Sentence case, no uppercase labels.
- Icons (lucide, via `components/ui/Icon.tsx`) only where a word would not
  fit: copy, delete, search, back, window controls.

## Components

`components/ui`: `Button` (primary, outline, ghost, danger), `IconButton`,
`Switch`, `Keys`, `CopyButton`, `Icon`.
`components/settings/controls.tsx`: `Page`, `Section`, `Row`, `Notice`,
`Select`, `Segmented`, and the setting controls (`ToggleSetting`,
`SelectSetting`, `SegmentedSetting`, `NumberSetting`) that load, save and
show the backend's error for one key.

- At most one primary (solid ink) button per view.
- Destructive actions confirm in place: the control becomes Cancel and a
  red button. No dialogs.
- State is a dot: solid for on/ready/in use, hollow for not, red for a
  problem.
