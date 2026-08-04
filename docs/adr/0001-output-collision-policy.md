# Output filename collision policy

When two outputs would write the same path, or an output would overwrite a selected source file, the batch **rejects at Start** (pre-flight) and reports the clash — it never silently picks one. When an output path already exists from a previous run, the batch **overwrites it silently**.

Re-running the tool into the same destination is the common case, so overwrite-on-rerun is the expected, ergonomic behavior. The two dangerous failure modes — two outputs fighting for one path (a rule configuration error), and clobbering an original source file (empty suffix into the source dir) — are caught up front by the pre-flight guards, so the only thing left to overwrite is prior-run output, which is intended.

## Considered options

- **Overwrite everything silently** — rejected: loses originals when suffix is empty and dest equals the source dir.
- **Skip existing files** — rejected: makes re-runs stale; a user tweaking a rule and re-running expects fresh outputs, not silently-kept old ones.
- **Prompt per file** — rejected: an `N×M` batch can be hundreds of files; per-file dialogs are unusable.
- **Pre-flight guards + overwrite prior-run** *(chosen)* — fails fast on the genuinely dangerous collisions, stays frictionless for the common re-run case.
