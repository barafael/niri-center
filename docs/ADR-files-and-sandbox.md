# ADR: where the app writes, and the sandbox around it

**Status:** accepted (sandbox 2026-10-03, session bus 2026-10-06, XDG
placement 2026-10-11). Held provisionally (docs/edit-epistemology.md [provisional]).

**Decision.** The app writes only to the config's own directory (and where
its symlink points) and to its folders where the XDG spec puts them, and
a Landlock sandbox holds it to that. The sandbox is on by default;
`NIRI_CENTER_NO_SANDBOX=1` is the escape hatch. Includes outside the
config's directory are read-only, and a save touching one stops before it
writes anything. Without a home folder nothing is written.

**Why.** A settings app runs with the user's rights over their dotfiles; the
kernel, not discipline, should keep it to the files it edits. The niri
socket, the Wayland socket and the session bus stay connect-only: without
them IPC, pop-out panes and the file chooser fail, and the bus adds no
power, since the niri socket already lets the app spawn programs outside
the sandbox. Landlock checks only files opened after it is applied, so it
waits until iced holds its GPU and display.

**Falsifiers.** A feature people need that cannot work inside the scope; a
write outside it that the sandbox test does not catch.
