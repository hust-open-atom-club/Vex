CORE_RISKS:
- `1000+ unit tests` is likely a vanity metric for a 2,378 LOC Rust CLI. It implies roughly one test per 2-3 LOC, which will mostly create brittle assertion noise unless the code is extremely logic-dense.
- Replacing all 52 `anyhow` sites at once is high-risk: it changes error propagation, messages, context, tests, and possibly CLI behavior in one broad patch.
- Typed errors can become over-modeled. Six broad categories are reasonable, but too many variants may just encode call-site trivia instead of stable domain failures.
- Remote operations are the riskiest area. Git shell-outs combine subprocess behavior, filesystem state, auth, network, remotes, and user input. A pure error refactor may not improve correctness unless remote behavior gets isolated and tested.
- The proposal risks optimizing internals without defining external compatibility: exact stderr text, exit codes, config file format, and behavior under corrupt/missing state.

MISSING_REQUIREMENTS:
- Define whether `vex` is a library API, CLI-only binary, or both. Typed errors matter much more if downstream Rust callers are expected.
- Define compatibility expectations for error messages. Are existing `anyhow::Context` strings user-facing contracts or freely changeable?
- Define exit-code behavior per error category, especially validation vs IO vs git failure vs serialization corruption.
- Add config migration/corruption cases: invalid JSON, missing fields, unknown fields, incompatible `qemu_version`, non-UTF8 paths/args if relevant.
- Add remote security requirements: malformed remote specs, shell injection resistance, path traversal, hostile branch names, hostile config names, and untrusted repo contents.
- Add concurrency requirements: simultaneous `save`, `rename`, `remove`, `push`, or `pull` touching the same config directory.
- Add platform requirements: Linux-only vs macOS/Windows behavior, path separators, executable lookup, permissions, and symlink handling.
- Add performance expectations for listing/loading many configs and for remote operations over large repos.

TECHNICAL_GAPS:
- Full `anyhow` removal is probably not the right first goal. A stronger target is: domain/library layers return `Result<T, VexError>`, while the CLI/top-level may keep `anyhow` or convert `VexError` into `anyhow::Error` for presentation.
- `VexError` should preserve source errors with `#[source]` where possible. Otherwise the migration will lose useful IO/serde/git context.
- Current `From<io::Error>` and `From<serde_json::Error>` may be too generic. Storage errors often need operation/path context; serialization errors need config name/path/context.
- Remote code likely needs an intermediate abstraction before good typed errors: e.g. `GitCommand`, `RemoteSpec`, `RemoteRepo`, or a small command-runner boundary.
- A 10-case dispatch in `lib.rs` is not automatically a problem. Refactoring command dispatch before defining error boundaries could add ceremony without payoff.
- Test architecture should come before test count. You need a matrix of behavior, state setup, and failure modes; otherwise typed error tests will just snapshot implementation details.

ALTERNATIVE_DIRECTIONS:
- Recommended: incremental typed error migration. Start with `config` and `validation`, then `storage`, then commands, then `remote`. Keep top-level `anyhow` until domain errors stabilize. Lower risk, easier review, preserves behavior.
- Behavior-first testing. Add focused integration tests around current CLI behavior before refactoring errors. This catches regressions in messages, exit status, config files, and git interactions.
- Remote boundary extraction. Pull git shell-out behavior behind a small interface or helper module, then test parsing and command construction separately from live git integration. Higher design effort, best payoff for the 431 LOC remote module.
- Property-based testing selectively. Use `proptest` for `RemoteSpec` parsing, config-name validation, argument round-trips, and path-like edge cases. Do not use it everywhere.
- Golden/snapshot tests for CLI output. Useful for `list`, `print`, validation errors, and common failure cases, but keep snapshots small to avoid churn.
- Smaller realistic target: maybe 80-200 meaningful tests total, mixing unit, integration, property, and regression tests. More only if generated from a clear matrix of externally relevant behaviors.

QUESTIONS_FOR_USER:
- Is the goal public API quality, better user-facing CLI errors, easier testing, or all three?
- Must existing error messages remain stable, or can they change if clearer?
- Should `anyhow` be banned entirely, or only below the CLI entry point?
- What platforms are supported?
- Are remote operations expected to handle hostile/untrusted repositories and config names?
- Do you want exhaustive coverage metrics, or risk-based confidence with maintainable tests?
- Should config writes be atomic and safe under concurrent commands?
- Is `1000+ tests` a hard requirement from someone, or just a placeholder for “much better coverage”?

CANDIDATE_CRITERIA:
- Domain modules expose `VexResult<T>` / `VexError`; top-level CLI may still use `anyhow` only for presentation glue.
- Error variants preserve source errors and relevant context such as path, config name, remote name, git command category, and validation field.
- Existing CLI happy-path behavior remains unchanged unless explicitly approved.
- Error output and exit-code behavior are documented and covered by integration tests.
- Config validation covers empty binary, invalid names, duplicate names, missing/corrupt files, malformed JSON, and unsupported remote specs.
- Remote tests cover parse failures, missing git, git command failure, nonexistent remote/branch, conflicting remote config, and unsafe input.
- Test suite uses a deliberate mix: unit tests for pure parsing/validation, integration tests for CLI workflows, property tests for parsers, and regression tests for known bugs.
- Test count is justified by coverage of behavior classes, not a numeric target.
