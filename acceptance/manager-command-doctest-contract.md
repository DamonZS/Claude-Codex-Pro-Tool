# Manager Command Documentation Contracts Acceptance

Spec: `spec/manager-command-doctest-contract.md`.

- [x] The Manager doctest target passes with zero failures.
- [x] The four private-helper examples are non-executable text and describe
      the actual output or behavior.
- [x] Unit coverage proves URL and auth sanitization remove sensitive values,
      error chains preserve root-to-context order, and failed results preserve
      the user message and payload.
- [x] No helper is made public for documentation testing.
- [x] No routes or unrelated source are changed.

Required evidence:

```text
cargo test -p claude-codex-pro-manager --doc
cargo test -p claude-codex-pro-manager --lib commands::tests::
```

Observed: doctests passed with `0 passed; 0 failed` (the four former doctests
now compile as documentation text), and the focused command test module passed
`70 passed; 0 failed; 2 ignored`.
