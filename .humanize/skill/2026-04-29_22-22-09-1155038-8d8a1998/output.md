AGREE:
- Typed `VexError` below the CLI presentation layer is the right direction.
- Keeping `anyhow` only at `main`/`lib.rs` initially is reasonable.
- Migrating `config` first is sensible because it is shared by most commands.
- Preserving underlying sources for IO, JSON, git, and process-launch failures is required.
- Pure test surfaces for remote parsing, config validation, name validation, and parameter substitution are worth extracting.
- Config-name validation is a real security need because current names can affect filesystem paths.

DISAGREE:
- `1000+ tests` should not be an acceptance criterion. Use behavior coverage, not generated-case counts.
- “All error paths” is too broad unless you define meaningful error classes. Exhaustively forcing every filesystem failure is low value.
- Integration tests cannot verify `VexError` variants. Binary tests can verify exit status and stderr; variant assertions belong in module/API tests.
- `Malformed JSON` and `missing required fields` do not belong inside `validate_config(&QemuConfig)`. They are deserialization/load errors.
- Whole-enum `PartialEq` for `VexError` is not a good target because `std::io::Error` and `serde_json::Error` are not meaningfully comparable.
- Remote “shell metacharacter” rejection is partly misframed: remote git calls use `Command` args. The real shell surface is `edit_command` using `sh -c` / `cmd /C`.

REQUIRED_CHANGES:
- Correct the test inventory: current repo has 30 tests, but not `22 integration + 8 unit`; it is 17 escargot-style command tests, 8 `substitute_params` tests, and 5 inline remote unit tests.
- Define the error taxonomy precisely before coding: either de-duplicate `StorageError` vs `IoError`, or state exactly when each is used.
- Decide whether to add `thiserror`; `#[source]` implies that style. Otherwise require manual `Error::source()` implementations.
- Add fields to variants now: operation, path, config name, remote ref/spec, git args/status/stderr/stdout, and QEMU `ExitStatus` or `Option<i32>` because a process may terminate without an exit code.
- Include `utils/io.rs` in the migration plan or explicitly classify it as CLI glue allowed to keep `anyhow`.
- Move config-name validation early, before broad command migration. Apply it through every name entry point: save, print, exec, rename, remove, push local name, and pull target name.
- Split config loading into typed functions such as `parse_config_json` / `load_config`; keep structural JSON failures separate from semantic `validate_config` failures.
- Make `save` and `edit` call semantic validation too; right now validation is not consistently applied.
- Replace the test-count target with a coverage matrix: command happy paths, representative CLI failures, module-level variant assertions, and property tests only where invariants are clear.
- Extract parameter substitution behind an injected env lookup before property testing; global environment mutation is flaky and unsafe in Rust 2024 tests.
- Move security hardening earlier than M9 if it changes path construction.
- Add an explicit final audit gate: `rg anyhow`, `rg "anyhow::bail|context\\("`, `cargo test`, and command-level smoke tests.

OPTIONAL_IMPROVEMENTS:
- Use `thiserror` to keep `Display` and source handling compact.
- Keep `anyhow` for top-level CLI reporting until typed errors stabilize; removing it can be a later cleanup.
- Move escargot binary tests to top-level `tests/` eventually, while keeping private pure-function tests inline.
- Use `matches!` and field assertions for `VexError` tests instead of `PartialEq`.
- Add one or two stderr substring tests per important CLI failure rather than snapshotting all wording.
- Consider a small config IO module that owns read/write/deserialize/serialize behavior so callers do not repeat JSON and filesystem error mapping.

UNRESOLVED:
- Is broad security hardening in scope for this refactor, or should it be a separate project?
- Are CLI error messages a stable contract, or only the error categories and exit behavior?
- Should `anyhow` remain permanently at the CLI boundary, or is full removal desired?
- Is adding `thiserror` acceptable?
- Is `1000+` a hard external requirement, or can it be replaced by behavior-class coverage?
