# Manager Command Documentation Contracts

## Background

The Manager crate's full test run currently fails in four doctests attached to
private logging and error-result helpers. The comments contain snippets that
Rust treats as executable doctests even though they reference private helpers
and undefined placeholder values.

## Goal

Make the four examples accurate documentation text that does not expose a
private helper solely for doctest access, and retain unit coverage for the
helpers' real sanitization and error-result behavior.

## Scope

- Update only the examples attached to `sanitize_url_for_logging`,
  `sanitize_auth_header`, `format_error_chain`, and `failed_with_context`.
- Add focused unit tests in the existing Manager command test module.
- Preserve the helper implementations and all command behavior.
- Do not modify routing code or reformat the file.

## Contracts

- URL logging removes query contents, fragments, and basic-auth credentials.
- Authorization values retain their scheme while replacing the credential.
- Error chains are rendered from root cause to outer context with ` → `.
- `failed_with_context` returns a failed result with the caller's message and
  payload, while recording the diagnostic chain.
- Documentation examples are text demonstrations and are not compiled as
  doctests for private functions.

## Verification

Run `cargo test -p claude-codex-pro-manager --doc` and the focused command
unit tests through the Manager library test target.
