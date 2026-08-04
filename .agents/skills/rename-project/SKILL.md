---
name: rename-project
description: Use when forking this template or otherwise giving it a new project identity — renames the project name across package.json, the Rust crate, Tauri's bundle identifier/window title, and any docs that mention the crate name, from a single new-name argument.
---

# Rename Project

## Overview

This template's identity lives in one name, expressed in two case conventions (kebab-case for package/bundle names, `snake_case` + `_lib` suffix for the Rust crate), spread across several files. A fixed file list will always miss something a particular fork added (a doc, a CI file) — the final step is always a grep sweep, not the table below.

## Usage

Takes one argument: the new project name, e.g. `acme-widget-pro`. Slugify it before using:
- **kebab-slug**: lowercase, spaces/underscores → hyphens, strip anything outside `[a-z0-9-]`
- **snake_lib**: kebab-slug with hyphens → underscores, plus `_lib` suffix (Rust identifiers can't contain hyphens)

## What to change

| File | Field | Value | Note |
|---|---|---|---|
| `package.json` | `name` | kebab-slug | |
| `src-tauri/Cargo.toml` | `[package].name` | kebab-slug | Cargo package names allow hyphens |
| `src-tauri/Cargo.toml` | `[lib].name` | snake_lib | |
| `src-tauri/src/main.rs` | the `<name>_lib::run()` call | snake_lib | must match `[lib].name` exactly or the build breaks |
| `src-tauri/tauri.conf.json` | `productName` | kebab-slug | this becomes the built bundle/installer file name |
| `src-tauri/tauri.conf.json` | `identifier` | replace only the **last** dot-segment with kebab-slug | the rest is the author's reverse-DNS namespace (e.g. `com.gmail.author`) — not part of the project name, leave it |
| `src-tauri/tauri.conf.json` | `app.windows[0].title` | kebab-slug | |

Then grep the whole repo (excluding `node_modules/`, `target/`, `dist/`, `.git/`) for the *old* kebab-slug and the old snake_lib name. Anything that still matches is a spot the table missed — commonly a `CLAUDE.md`/`README.md`/`AGENTS.md` prose mention, a CI workflow file, or a `.vscode` config.

## Do NOT rename

Generic tech-stack descriptions aren't the project's identity — leave them: `README.md`'s heading, `index.html`'s `<title>` (browser-tab title, separate from the native window title above), and any "Welcome to X + Y + Z!" boilerplate in the UI. Also leave author/copyright fields (`Cargo.toml`'s `authors`, `LICENSE`) and the reverse-DNS namespace prefix in `identifier` — a project rename isn't an ownership transfer.

## Don't hand-edit Cargo.lock

`Cargo.lock` has its own `name = "..."` entry mirroring `Cargo.toml`. Don't touch it directly — after editing `Cargo.toml`, run `cargo check` in `src-tauri/` and it regenerates correctly (including re-sorting the entry alphabetically).

## Verify

- `pnpm install` (only needed if `package.json`'s name changed the lockfile — check `git diff` after)
- `pnpm exec tsc --noEmit` && `pnpm exec vite build`
- `cargo check` in `src-tauri/` (add `~/.cargo/bin` to `PATH` first if `cargo` isn't found)

## Common Mistakes

| Mistake | Fix |
|---|---|
| Renaming `[lib].name` but not the call site in `main.rs` | Names must match exactly — Rust won't compile otherwise |
| Using the raw argument (spaces, mixed case, punctuation) directly as a Cargo/crate name | Slugify first: kebab for package names, `snake_case` for the Rust lib |
| Hand-editing `Cargo.lock`'s name entry | Let `cargo check` regenerate it |
| Renaming the entire `identifier` string | Only the last segment is the project name; the rest is the author's namespace |
| Stopping after the file table above | Always grep for the old name afterward — forks add files the table can't predict |
| Renaming stack-description text (README heading, "Welcome to X+Y+Z" UI copy) | That describes the tech stack, not the project — leave it |
