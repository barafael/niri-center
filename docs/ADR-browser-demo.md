# ADR: the app also runs in a browser

**Status:** accepted 2026-10-06. Held provisionally (AGENTS.md
[provisional]).

**Decision.** The app also runs in the browser, published with GitHub
Pages, on niri's default config, without niri or a disk. It draws with
WebGPU, else WebGL, else tiny-skia.

**Why.** Trying the app should not need niri, or trust in a binary.

**Rejected alternatives.**

- WebGPU alone: release browsers on Linux do not have it yet. Firefox: "For
  other platforms such as Linux and macOS on Intel Silicon it is enabled in
  nightly" (MDN, Experimental features in Firefox, checked 2026-10-28).
  Chrome: "carefully rolling out WebGPU for Linux, starting with support
  for Intel Gen12+ GPUs" (New in WebGPU 144). Observed 2026-10 on one AMD
  laptop: Chrome 155 offered `navigator.gpu` with no adapter, and a
  WebGPU-only build crashed there.
- tiny-skia alone: 14 to 175 ms a frame at 1280×900 and up to 480 ms at
  twice the pixel density, against 1 to 3 ms on the GPU.
- MSAA over WebGL: it left a page's canvas drawings on the next page and
  lost icons (iced-rs/iced#2400). Without it, diagonal icon strokes are
  slightly jagged.
- Shadows under tiny-skia: it draws blurred shadows as slabs it never clips
  or repaints.
- Optimizing the whole build for speed: 9% more download, no faster frame
  on the GPU.

**Falsifiers.** A page that cannot be shown honestly without niri; the
browser build holding back a desktop feature; Firefox and Chrome releases
offering WebGPU on Linux for AMD and Intel alike, which makes the WebGL
path worth reweighing.
