# AGENTS.md — bootstrap

This file bootstraps the real AGENTS.md. Instead of shipping opinions the user
never asked for, your **first session** in this project runs a short interview
and rewrites this file into the project's actual conventions.

## First session: run the interview

If this file still contains the "Engineering rules" section kept verbatim, the
interview has not happened yet. Ask everything in **one round** (not one message
per question), accept short answers, and skip any question the user answers
with "default".

1. **Commit style** — prefix convention (`feat:` / `fix:` / `chore:` / none)?
   Message language? Subject-line rules (length, imperative, capitalization)?
2. **Collaboration style** — plan first and wait for approval, or act directly?
   Where is the line between "just do it" and "ask first"? How much refactoring
   is welcome while touching nearby code?
3. **Language** — for conversation, code comments, docs, and UI strings?
4. **Delivery bar** — besides `npm run lint` and `npx tsc --noEmit`, what must
   pass before work counts as done (tests, `cargo check`, packaging smoke)?
5. **Anything else** — pet peeves, forbidden actions, naming or structure
   preferences worth writing down?

Then rewrite this file:

- Keep the file name `AGENTS.md` and keep the `## Engineering rules` section
  intact — translate it only if the user asked for another language.
- Structure the rest around the interview answers.
- Zero filler. No platitudes ("write clean code", "be concise"), nothing the
  template already enforces by construction. Every line must change how you act.
- The interview is one-time. Once this bootstrap section is gone, never ask again.

## Engineering rules

Three rules that normal, well-scoped tasks would otherwise silently violate.
Keep them intact.

- **Dependency partition**: `dependencies` in package.json stays empty — the
  Vite frontend bundles everything, so all JS libraries go to
  `devDependencies`. Rust dependencies live in `src-tauri/Cargo.toml` and are
  added only when actually used.
- **Frontend never touches system capabilities directly**: all native calls go
  through `@tauri-apps/api` (`invoke` / `listen`, see `src/lib/tauri.ts`).
  New commands: `#[tauri::command]` in `src-tauri/src/lib.rs`, registered in
  `generate_handler!`, permission entries in `src-tauri/capabilities/` when
  needed.
- **Definition of done**: `npm run lint` (zero warnings), `npx tsc --noEmit`,
  and `npm test` all pass before work counts as complete; run `cargo check`
  in `src-tauri/` whenever Rust code or capabilities change.
