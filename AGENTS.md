# Rules for agents

- Never silence a lint. Change the code.
- Never write a comment, except the doc comment a lint requires on a public item. Never write a
  doc attribute in its place. Comments go stale: `// keep 3 weeks` above code that keeps 5
  misleads the next agent. Use names: `const WEEKS_KEPT: usize = 5;`
- Write each doc comment for the person who calls the item. Say what it is for and what the
  caller must know. Never restate the name.
- Runnable binaries go in `apps/`, shared libraries in `crates/`.
- Run `just check` before you finish. A commit is refused while it fails.
- Never change a failing test to make it pass: fix the code, or stop and tell the owner which
  test and why.
- Lints, dependency rules, the `justfile`, the toolchain, CI and these rules ask the owner
  before an edit. Do not work around the ask.
- Once per clone, run `just hooks` to turn on the pre-commit check.
