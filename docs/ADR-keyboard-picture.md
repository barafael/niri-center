# ADR: the keyboard picture comes from xkb

**Status:** accepted 2026-10-27. Held provisionally (AGENTS.md
[provisional]).

**Decision.** The Keybinds page draws each layout the config lists from its
`xkb` settings, compiled with libxkbcommon as niri compiles them, and shows
the one niri has active. The keyboard's shape (ANSI for `us`, ISO
otherwise) is a guess no source reports.

**Why.** niri matches binds against the keysyms of its xkb keymap, so the
same compilation is the one source that agrees with niri. Drawn pictures
cover two layouts and drift.

**Rejected alternatives.** Asking the hardware: keyboards report key
positions, not what their caps say, and USB's country code is almost
always 0. Reading the keymap niri sends over Wayland: exact, keymap `file`
included, but it needs a second Wayland connection, since winit does not
pass the keymap on.

**Falsifiers.** A layout whose picture disagrees with what niri binds; a
keymap `file` user missing their layout.
