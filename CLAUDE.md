# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

Tauri 2 + React 19 + TypeScript + TailwindCSS 4 + shadcn/ui desktop app template. Frontend in `src/`, Rust backend in `src-tauri/`. Package manager is **pnpm** (not npm/yarn).

## Commands

```bash
pnpm install          # install deps
pnpm dev               # vite dev server only (usually not needed directly, see below)
pnpm tauri dev         # run the full desktop app (spawns vite dev server via beforeDevCommand)
pnpm build              # tsc typecheck + vite build (frontend only)
pnpm tauri build       # build the full desktop app bundle
pnpm lint               # oxlint --fix src
pnpm format             # prettier -w --cache src
pnpm ui <command>       # shadcn CLI, e.g. `pnpm ui add dialog` to add a component
```

No test runner is configured in this template.

Vite dev server is fixed to port `1420` (required by `tauri.conf.json`'s `devUrl`) and fails to start if the port is unavailable.

## Architecture

- **Frontend** (`src/`): standard Vite React app. Entry `src/main.tsx` → `src/App.tsx`. Path alias `@/*` maps to `src/*` (configured in both `tsconfig.json` and `vite.config.ts`).
- **Backend** (`src-tauri/`): Rust crate `image_batch_resizer_lib` (note the `_lib` suffix, required so the lib and bin crate names don't collide on Windows). `src-tauri/src/main.rs` is the binary entry point that just calls `run()` from `lib.rs`, which is where Tauri commands/plugins get registered.
- Frontend and backend talk over Tauri's IPC; the CSP in `tauri.conf.json` (`app.security.csp`) explicitly allows `ipc:`/`ipc.localhost`. New Tauri commands need a matching permission added to `src-tauri/capabilities/default.json` (currently empty `permissions: []`).
- Release builds are tuned for small binary size: `opt-level = "s"`, LTO, `panic = "abort"`, stripped symbols (`src-tauri/Cargo.toml [profile.release]`).

## shadcn/ui conventions

- Style: `new-york`, base color `slate`, CSS variables enabled, icon library `lucide-react`.
- Global styles/theme live in `src/App.css` (this is the `tailwind.css` target in `components.json`, not a separate globals file).
- Add new components via `pnpm ui add <component>` rather than hand-writing them, so they land in `src/components/ui/` with the project's aliasing (`@/components`, `@/lib`, `@/hooks`).
- Utility class merging goes through `cn()` in `src/lib/utils.ts` (`clsx` + `tailwind-merge`), used by every shadcn component's `className` prop.

## Linting & formatting

- Lint: `oxlint` (`.oxlintrc.json`), plugins: import, jsx-a11y, oxc, react, react-perf, typescript, unicorn.
- Format: `oxfmt` (`.oxfmtrc.jsonc`), Rust-based formatter from the same oxc toolchain as `oxlint`. `printWidth: 100`, single quotes, `sortImports: true` (perfectionist-style default groups), `sortTailwindcss` pointed at `src/App.css` for class sorting on the `cn()` helper.
- Both `lint` and `format` scripts only target `src` (not `src-tauri`).
