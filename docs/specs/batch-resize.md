# Spec: Batch Resize

Status: ready-for-agent

## Problem Statement

A user with a folder of images — photos, product shots, app icons — frequently needs the same set of images produced at several fixed sizes and formats: a thumbnail, a social card, an app icon, a print-ready copy. Doing this by hand per image, per size, in a general-purpose editor is tedious and error-prone. There is no quick, repeatable way to say "take these images, produce each of them at these exact sizes and formats, once."

## Solution

A desktop application where the user selects a set of source images, picks an output directory, defines a list of resize rules (each a fixed width, height, format, and filename suffix), and presses Start. The application applies every rule to every source — `N` sources × `M` rules outputs — resizing each to the exact rule dimensions at best quality, handling transparency correctly, and writing each output to the destination with a derived filename. Progress is shown live; the run can be stopped; a summary reports the result.

## User Stories

1. As a content creator, I want to select multiple source images at once, so that I can resize a whole batch in one run.
2. As a content creator, I want to select only PNG and JPEG files as sources, so that I work with formats the tool fully supports.
3. As a content creator, I want to choose a destination directory for the outputs, so that I control where files are written.
4. As a content creator, I want to browse for the destination directory rather than type a path, so that I avoid path mistakes.
5. As a content creator, I want the Start button disabled until I have picked sources, at least one rule, and a destination, so that I cannot start an incomplete run.
6. As a content creator, I want to add resize rules one at a time, so that I can build up the set of output sizes I need.
7. As a content creator, I want each rule to specify an exact width and height in pixels, so that outputs are a precise, predictable size.
8. As a content creator, I want each rule to specify an output format (PNG or JPG), so that I get the file type each target consumes.
9. As a content creator, I want each rule to specify a filename suffix, so that outputs from different rules do not overwrite one another.
10. As a content creator, I want to remove a rule with a single click, so that I can correct mistakes while configuring the batch.
11. As a content creator, I want the rules shown in a table with Width, Height, Format, Suffix, and a remove control, so that the whole configuration is visible at a glance.
12. As a content creator, I want the image resized to exactly the rule's width and height ignoring aspect ratio, so that the output matches a fixed slot exactly.
13. As a content creator, I want upscaling allowed (rule larger than source), so that I can produce a required size even from a small source.
14. As a content creator, I want resizing done at the best available quality, so that outputs look as good as the resolution allows.
15. As a content creator, I want transparent sources saved as transparent PNGs, so that cutouts keep their transparency.
16. As a content creator, I want transparent sources saved as JPGs with a white background, so that the JPG (which has no alpha) looks clean on white.
17. As a content creator, I want opaque sources to stay opaque in either format, so that no spurious transparency is introduced.
18. As a content creator, I want output filenames derived as `{stem}{suffix}.{ext}` with a lowercase extension, so that files are named predictably and consistently.
19. As a content creator, I want to be stopped from starting a batch whose outputs would collide with each other, so that I do not silently lose outputs to a naming clash.
20. As a content creator, I want to be stopped from starting a batch that would overwrite my selected source files, so that my originals are never destroyed.
21. As a content creator, I want a re-run into the same destination to overwrite the previous outputs, so that tweaking a rule and re-running gives me fresh results without manual cleanup.
22. As a content creator, I want a live progress indicator during the run, so that I know the tool is working and roughly how far along it is.
23. As a content creator, I want to stop a running batch, so that I can abort a mistaken or oversized run.
24. As a content creator, I want already-written outputs kept when I stop, so that a stopped run is not a total loss.
25. As a content creator, I want one corrupt or unreadable source to be skipped, not abort the whole batch, so that a single bad file does not waste the good ones.
26. As a content creator, I want a summary when the batch finishes, so that I know how many outputs succeeded, failed, or were skipped.
27. As a content creator, I want the application to use my system's light or dark color scheme, so that it fits my desktop.
28. As a content creator, I want the resize work parallelized across my CPU cores, so that large batches finish in a reasonable time.
29. As a content creator, I want every output produced from a single source decoupled from the others, so that one failed decode does not block the rest of that source's outputs.

## Implementation Decisions

The work splits into a Rust backend that owns all domain logic and a React frontend that is thin UI glue over Tauri IPC. Per the project glossary (`CONTEXT.md`), the core nouns are **Source File** (a PNG/JPEG input), **Resize Rule** (a fixed size + format + suffix), and **Output File** (one rule applied to one source).

### Rule and plan model

A resize rule carries both dimensions as required positive integers, a format, and a suffix. The resolved type shape (this is the agreed model, not a prototype):

```rust
enum Format { Png, Jpg }

struct ResizeRule {
    width: u32,   // required, > 0
    height: u32,  // required, > 0
    format: Format,
    suffix: String,
}
```

Applying the rules to the sources produces an ordered **plan**: for `N` sources × `M` rules, `N * M` output items, each resolving to a concrete output path `{dest}/{stem}{suffix}.{ext}` where `stem` is the source filename stem (everything before the final `.` — so `my.photo.png` → `my.photo`), `suffix` is the rule's suffix, and `ext` is the rule's format's lowercase extension (`png` / `jpg`).

### Pre-flight collision guard (ADR-0001)

Before any file is written, the plan is checked:

- **Intra-batch duplicates** — two output items resolving to the same path (caused by a shared stem across sources, duplicate suffix+format across rules, or an empty suffix). The run is rejected with a message naming the clash.
- **Source-overwrite guard** — any output path that equals one of the selected source files. The run is rejected, protecting originals.
- **Pre-existing outputs** in the destination are overwritten silently; this is the intended re-run behavior, and the two guards above remove the dangerous cases. See `docs/adr/0001-output-collision-policy.md`.

### Resize, quality, and transparency

Each output item is produced by a pure resize function: decode the source, resize to exactly `width × height` with the `image` crate's `Lanczos3` filter (the "best quality" choice), then encode. Aspect ratio is ignored — the image is stretched to the exact dimensions, and upscaling is allowed with no cap.

Transparency is handled at encode time only:

- **JPG output** — the decoded RGBA image is composited onto an opaque white background, then JPEG-encoded. Hardcoded white, not configurable.
- **PNG output** — the alpha channel is passed through unchanged (opaque sources stay opaque; no alpha is fabricated).

JPEG quality is a hardcoded `90`. These constants (`Lanczos3`, JPEG `90`, white flatten) are not exposed in the UI.

### Execution: parallelism, progress, cancellation, errors

The plan is executed in parallel across CPU cores (the `rayon` crate) — resizing is embarrassingly CPU-bound work. A cancellation flag is checked between outputs; the Stop command flips it, after which remaining items are skipped and already-written outputs are kept. A shared counter emits a progress event (completed / total) per finished item, which the frontend renders as a progress bar. A per-item failure (corrupt image, decode error, disk error) is recorded and skipped; it never aborts the batch. On completion (natural or stopped) a finished event carries a summary: counts of succeeded, failed, and skipped outputs.

### Tauri IPC contract

- Source and destination selection use the Tauri **dialog** plugin directly from the frontend (multi-select file dialog filtered to PNG/JPEG; directory dialog for the destination) — no custom commands for picking.
- A **start** command takes the selected source paths, the rules, and the destination. It runs the pre-flight guard; on a clash it returns an error and writes nothing. On success it spawns the parallel run and emits `progress` and `finished` events.
- A **stop** command flips the cancellation flag for the running batch.
- The dialog plugin permission must be added to the desktop capability set (currently empty).

### Frontend

Column layout, top to bottom: source files selector, destination directory (input + browse), resize rules (an add button above a table with Width / Height / Format / Suffix / a remove control), and a submit button labeled **Start** when idle and **Stop** when running. The Start button is disabled until sources, at least one valid rule, and a destination are all present. A progress bar appears during the run. On the finished event, a dialog shows the summary. The UI follows the system light/dark scheme via the existing shadcn/theme setup.

## Testing Decisions

One automated seam: **Rust unit tests via `cargo test`** over pure, Tauri-free functions. No JavaScript test runner exists in the project and none is added; the frontend is verified manually via the run skill.

A good test here checks external behavior of the pure domain functions — given inputs, what plan / collision verdict / output bytes result — never their internal structure. Three modules are tested:

- **`plan_outputs`** — `N` sources × `M` rules yields `N * M` items with correctly derived paths: lowercase extension, suffix insertion, multi-dot stem handling (`my.photo.png` → `my.photo`), and upscaling/ignore-aspect cases produce items at the exact rule dimensions.
- **`detect_collisions`** — asserts the ADR-0001 guards: intra-batch duplicate paths are flagged, source-overwrite paths are flagged, and legitimate prior-run overwrites are not flagged (collision detection is independent of what already exists on disk; only the plan + source set matter).
- **`resize_one`** — with tiny committed fixtures (a small RGBA PNG and a small JPEG): resizing hits exact target dimensions; a transparent PNG → JPG output has no alpha and a white background where it was transparent; a transparent PNG → PNG output retains alpha; an opaque source → PNG stays opaque. JPEG quality and the Lanczos filter are exercised implicitly through these.

Prior art: none — the template ships no tests. `cargo test` requires no additional infrastructure, which is why it is the chosen seam. Fixtures live alongside the Rust tests.

## Out of Scope

- Input formats beyond PNG and JPEG (WebP, GIF, BMP, TIFF, etc.).
- Folder selection or drag-and-drop of files/folders — multi-select files only.
- Persisting resize rules across sessions (rules start empty each launch).
- Exposing resize filter, JPEG quality, or background color as user settings.
- ICC / color-profile management.
- Animated or multi-frame formats.
- A per-file status table in the UI (the summary dialog carries aggregate counts only).
- Release bundling / CI pipeline changes (`bundle.active` remains false).
- Setting up a git remote or issue tracker (deferred; this spec is local markdown until that exists).

## Further Notes

- Domain glossary lives in `CONTEXT.md`; the collision policy rationale is recorded in `docs/adr/0001-output-collision-policy.md`.
- New backend dependencies: the `image` crate (decode/resize/encode) and `rayon` (parallel execution); the Tauri dialog plugin for file/directory picking. These follow the project's bare-major-version Cargo convention.
- The crate is `image_batch_resizer_lib` (the `_lib` suffix is required on Windows); commands register in the existing `run()` entry point.
- Publishing to the issue tracker with the `ready-for-agent` label is deferred until `/setup-matt-pocock-skills` has been run and a git remote is configured.
