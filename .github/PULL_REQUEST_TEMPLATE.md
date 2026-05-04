<!--
Thanks for contributing to crab! To speed up review:
- Use a Conventional Commits prefix: `feat:` / `fix:` / `refactor:` / `docs:` / `test:` / `chore:` / `build:` / `ci:`
- Describe what the code change does. Don't reference plan steps like "Phase 3 step 5".
-->

## Summary
<!-- 1-3 sentences on what this PR does and why. -->

## Related issues
<!-- Closes #123 / Refs #456. Use N/A if there are none. -->

## Type of change
<!-- Tick all that apply. -->
- [ ] 🐛 Bug fix
- [ ] ✨ New feature
- [ ] 🧹 Refactor (no behavioural change)
- [ ] 📝 Docs only
- [ ] ✅ Tests
- [ ] 🔧 Build / CI / dependencies
- [ ] 💥 Breaking change

## How was this tested?
<!-- List the commands you ran, new / changed test names, and any manual verification steps. -->
- [ ] `cargo fmt --all --check`
- [ ] `RUSTFLAGS="-Dwarnings" cargo clippy --workspace`
- [ ] `cargo nextest run --workspace`
- [ ] For TUI changes: tested on Git Bash / Windows Terminal / cmd.exe / Linux WSL (as relevant)

## Screenshots / recordings
<!-- For TUI / rendering changes, please attach a before/after image. Drag-and-drop into the editor works. -->

## Checklist
- [ ] Commit messages describe the code change; no plan-step or "Phase X" references.
- [ ] Comments explain *why*, not *what*.
- [ ] No unnecessary fallback or defensive code introduced.
- [ ] No leftover debug `println!` / `dbg!`.
- [ ] Docs updated if architecture changed (`docs/architecture.md` / `CLAUDE.md`).
- [ ] No `claude-code-best` / CCB references in code (docs only).
