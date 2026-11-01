# ADR: saved a moment after the change, with a history of exact texts

**Status:** accepted 2026-10-08, replacing review-before-apply and Preview;
amended 2026-10-09, 2026-10-11 and 2026-10-30. Held provisionally (docs/edit-epistemology.md
[provisional]).

**Decision.** An edit is saved a moment after it is made, once niri accepts
the whole config, and whole or not at all. A file another program changed
meanwhile is not written over. History keeps every version's
exact text, edits by other programs included, and restores a version as
one new change. Display changes go back after a short trial unless kept,
and only a click keeps one: Enter goes back as Escape does, since a black
or garbled screen invites a reflex (KScreen focuses Revert likewise).
"Save when I press Save" is the opt-in for people who want the diff first.
What became of the latest change, and Undo, sit in a bar along the
window's bottom on every page, as Undo takes back the latest change
wherever it was made. History keeps the version open beside the list, its
title and Restore in view however far the list scrolls.

**Why.** Reviewing before applying added a step most changes do not need;
History shows the diff afterwards, and Undo is one click.

**Rejected alternatives.** Review-before-apply and Preview: see above.
Per-change undo in History beside "go back to before": it confused people,
hence versions. jj-lib for the history: 193 crates against the app's 275,
a 0.x API that breaks monthly, and no wasm build. Undo in the header: the
user preferred it at the bottom (2026-10-30). A footer per page as in KDE's
System Settings: it reads as acting on that page, and Undo does not. The
diff left of the list in History: every other list in the app, and KDE's
guidelines, put what is picked from on the left.

**Falsifiers.** People miss checking a change before niri sees it (History
shows the diff only afterwards). People look for Undo at the top and miss
the bar at the bottom.
