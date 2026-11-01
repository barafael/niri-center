# ADR: colors, copy and layout

**Status:** accepted 2026-10-04; amended 2026-10-06, 2026-10-08,
2026-10-11 and 2026-10-30. Held provisionally (docs/edit-epistemology.md [provisional]).

**Decision.**

- **Colors** start from niri's own defaults (focus ring `#7fc8ff`, border
  `#ffc87f`), dark and light. Every text clears WCAG AA, control outlines
  3:1 (WCAG 1.4.11), and a high-contrast variant (AAA text) follows the
  desktop's contrast setting.
- **Copy** is sentence case, with no em dashes, emoji or decorative glyphs.
  Buttons say what happens. Copy carries data, niri's words or what a
  control does, and never narrates the screen.
- **Settings** show niri's own value when unset, where a value is written,
  and what overrides it. Where the system knows the answer there is a
  picker, with text as the fallback, and a field accepts exactly what niri
  accepts.
  A setting the config sets is marked at its edge, always, as KDE marks
  changed settings on request. A page of settings alone can go back to
  niri's defaults for all of them as one change, offered while the config
  sets any.
- **Navigation** is a sidebar at every width, and every page has a place
  in it. Forms open as dialogs, with their buttons at the right and the
  way out last. The config text a page edits shows only when asked for,
  from a button at the foot of the sidebar beside Help: beside the page,
  or in a window of its own where there is no room beside it. Help leads
  to niri's own documentation for the page.
- **Layout** does not rearrange as the window resizes. A part goes by the
  width it gets, and every page works in a third-width niri column (about
  620 px).
  A page sits in the middle of the room it has, as KDE's form layout
  centers settings, so a wide window has room on both sides rather than
  one.

- **Motion** in the app is niri's own, read from the config: a switch
  moves as niri moves a window, a notice drops in as niri's config
  notification does, and the config's slowdown applies. With animations
  off in niri, nothing in the app moves. A display change on trial floats
  over the top of the window instead of pushing the page down.

**Rejected alternatives.** Tabs between pages, and previous/next: tried and
dropped as disorienting. A sentence under each page title: it restated the
sidebar and the content, and GNOME and KDE have none. iced's own button
styles: they fade disabled labels to half opacity, below AA.

The config button beside each page title: its place moved with the
page's width, and narrow windows had none. A rail of icons at the right
edge for it: a second rail costs width at every size.

**Falsifiers.** People using the app report the line links as noise, or the
copy rules force worse wording than they prevent. People find niri's
bouncier springs distracting in a settings window, or miss the trial card
because it covers the page's top.
