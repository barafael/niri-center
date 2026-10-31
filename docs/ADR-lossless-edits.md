# ADR: edits splice the original text

**Status:** accepted; amended 2026-10-27. Held provisionally (AGENTS.md
[provisional]).

**Decision.** Every edit splices the original text, where niri reads it;
nothing reformats or re-serializes. Rewriting a comment or a third-party
block is a bug. `niri validate` decides what niri's dialect is. The one
rewrite is a window rule saved from its form, so the form opens only rules
it can say in full.

**Why.** People keep hand-written configs, shell-generated blocks (Noctalia,
DankMaterialShell) and comments in the same files; a settings app that
re-serializes them destroys what it does not manage.

**Falsifiers.** A round-trip test that fails on a real config; niri accepting
a config the parser rejects, or the reverse.
