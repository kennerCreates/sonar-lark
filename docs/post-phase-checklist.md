# Post-Phase Checklist

After completing each implementation phase:

1. **Update documentation**: Review and update `TODO.md`, `ARCHITECTURE.md`, and `CLAUDE.md` with any new types, patterns, or conventions introduced.
2. **Review warnings**: Run `cargo build` and review all warnings. Fix any that indicate real issues (unused imports, unnecessary mut, etc.). Warnings for types/functions that are planned for upcoming phases in the current sprint are acceptable and should be left alone — do not suppress them with `#[allow(dead_code)]`.
3. **Run clippy**: Run `cargo clippy -- -D warnings` and fix all lints. Clippy catches idiomatic issues, performance pitfalls, and common mistakes that `rustc` alone misses.
4. **Run tests**: Run `cargo test` and verify all tests pass. Add tests for new pure-logic functions.
5. **Manual testing feedback form**: After all automated checks pass, present a structured feedback form for manual testing. The form must include:
   - A checklist of every manually-testable behavior introduced or changed in the phase (specific actions, expected results).
   - Edge cases and error scenarios to verify (e.g., invalid input, rapid state transitions, boundary values).
   - Performance observations to watch for (frame drops, hitches, visual artifacts).
   - A "Pass / Fail / Notes" column for each item so the user can report results inline.
   - Regression checks: key existing behaviors that should still work unchanged.

   Format the form as a markdown table or checklist that can be filled out directly in chat.
