# Typed Error Architecture With Comprehensive Test Suite For Vex

## Goal Description

Systematically refactor the vex codebase (~2,378 LOC Rust CLI) to replace all `anyhow` usage in domain layers with typed `VexError` variants using `thiserror`, extract pure logic functions for filesystem-free testing, harden input validation and security, and build a comprehensive test suite combining unit tests, property-based tests (proptest), and integration tests covering all 10 commands, all error paths, and edge cases. The test strategy prioritizes behavior-class coverage over raw test counts.

## Acceptance Criteria

Following TDD philosophy, each criterion includes positive and negative tests for deterministic verification.

- AC-1: VexError enum expanded with rich context fields and `thiserror` derive
  - Positive Tests (expected to PASS):
    - Constructing each VexError variant with appropriate fields succeeds
    - `Display` output for each variant produces a user-friendly message
    - `Error::source()` returns the wrapped inner error for IoError, ConfigParseFailed, ConfigSerializeFailed, QemuLaunchFailed, GitCommandFailed
    - `VexError::from(std::io::Error)` produces an IoError variant
  - Negative Tests (expected to FAIL):
    - Attempting to construct IoError without a path and operation is a compile error
    - A `match` on VexError that omits any variant is a compile error (exhaustiveness)
  - AC-1.1: Config existence errors — ConfigNotFound{name}, ConfigAlreadyExists{name}
    - Positive: ConfigNotFound displays "Configuration 'x' not found"
    - Negative: ConfigNotFound with empty name still produces valid Display output
  - AC-1.2: Parse/Serialization errors — ConfigParseFailed{source}, ConfigSerializeFailed{source}
    - Positive: ConfigParseFailed wraps serde_json::Error, source() returns it
    - Negative: ConfigParseFailed cannot be constructed without a serde_json::Error source
  - AC-1.3: IoError{path, operation, source} — all filesystem operations
    - Positive: IoError with path="/tmp/x.json", operation="read" displays both
    - Negative: IoError does not silently drop the source io::Error
  - AC-1.4: Execution errors — QemuLaunchFailed{binary, source}, QemuExitError{binary, exit_code: Option<i32>}
    - Positive: QemuExitError with exit_code=Some(1) displays the code; None displays "unknown"
    - Negative: QemuExitError does not panic on None exit code
  - AC-1.5: Remote errors — RemoteNotConfigured, RemoteSpecInvalid{input, reason}, GitCommandFailed{args, stderr, stdout, exit_code}, RemoteConfigNotFound{id, name, tag}
    - Positive: GitCommandFailed preserves stderr and stdout content in Display
    - Negative: RemoteSpecInvalid rejects empty input string
  - AC-1.6: ValidationError{field: Option<String>, reason} — semantic validation
    - Positive: ValidationError with field=Some("qemu_bin") displays field name
    - Negative: ValidationError with field=None still produces a readable message
  - AC-1.7: EditorFailed{editor, exit_code: Option<i32>}
    - Positive: EditorFailed displays editor name and exit code
    - Negative: EditorFailed does not panic when exit_code is None

- AC-2: Incremental migration — domain modules use VexResult<T> = Result<T, VexError>
  - Positive Tests (expected to PASS):
    - `config_dir()` returns `VexResult<PathBuf>`
    - `save_command(...)` returns `VexResult<()>`
    - `clone_remote_repo()` returns `VexResult<(TempDir, PathBuf)>`
    - `lib.rs::run()` converts VexError to anyhow::Error at the CLI boundary
  - Negative Tests (expected to FAIL):
    - No `anyhow::bail!` or `.context()` remains in config/, commands/, or remote/ modules
    - `rg "anyhow::" src/config/ src/commands/ src/remote/` returns zero matches (except re-exports)
  - AC-2.1: config/ migrated (storage.rs, validation.rs)
  - AC-2.2: commands/ migrated (all 10 command modules)
  - AC-2.3: remote/mod.rs migrated with GitCommandFailed preserving stderr/stdout
  - AC-2.4: lib.rs run() uses `impl From<VexError> for anyhow::Error` or explicit mapping
  - AC-2.5: utils/io.rs remains with anyhow (CLI glue, 2 prompt functions only)

- AC-3: Config loading pipeline with clear separation
  - Positive Tests (expected to PASS):
    - `parse_config_json(valid_json)` returns Ok(QemuConfig)
    - `load_config("existing-name")` returns Ok(QemuConfig) from filesystem
    - `validate_config(&valid_config)` returns Ok(())
    - `validate_config_name("my-vm")` returns Ok(())
    - `save_command` calls `validate_config` after constructing QemuConfig
    - `edit_command` calls `validate_config` after editing
  - Negative Tests (expected to FAIL):
    - `parse_config_json("invalid json")` returns Err(ConfigParseFailed)
    - `parse_config_json("{}")` returns Err (missing required fields)
    - `load_config("nonexistent")` returns Err(ConfigNotFound)
    - `validate_config(&config_with_empty_binary)` returns Err(ValidationError)
    - `validate_config_name("")` returns Err(ValidationError)
    - `validate_config_name("../escape")` returns Err(ValidationError)
    - `validate_config_name("a/b")` returns Err(ValidationError)
  - AC-3.1: parse_config_json(content: &str) is pure, no I/O
  - AC-3.2: load_config(name: &str) combines I/O + parse
  - AC-3.3: validate_config expanded: empty binary, invalid args patterns
  - AC-3.4: validate_config_name: rejects path separators, empty, null bytes, ".", "..", excessively long names (>255 chars)
  - AC-3.5: save_command and edit_command both call validate_config consistently

- AC-4: Parameter substitution extraction
  - Positive Tests (expected to PASS):
    - `substitute_params(args, |k| env.get(k))` resolves known variables
    - Undefined variables remain as `${VAR_NAME}` literal
    - Mixed defined/undefined variables in one arg work correctly
    - Empty args vec returns empty vec
  - Negative Tests (expected to FAIL):
    - Tests do NOT use `std::env::set_var` (unsafe in Rust 2024 edition)
    - Malformed `${` without closing `}` is not substituted (passes through)
  - AC-4.1: substitute_params takes env lookup closure
  - AC-4.2: Default production caller passes std::env::var
  - AC-4.3: Tests use deterministic HashMap-based env lookup

- AC-5: Test coverage matrix (behavior-class coverage, not raw count)
  - Positive Tests (expected to PASS):
    - `cargo test` passes all tests (existing + new)
    - Each VexError variant has at least one construction + Display test
    - Each pure function has unit tests covering key paths
    - Each command has integration tests for happy path + primary error path
  - Negative Tests (expected to FAIL):
    - No test uses `unsafe { std::env::set_var(...) }`
    - No test depends on global mutable state
  - AC-5.1: Error variant unit tests — constructibility, Display, source() chains
  - AC-5.2: Config validation unit tests — empty binary, invalid names, valid configs, boundary cases
  - AC-5.3: Remote spec parsing unit tests — valid specs, path traversal, special chars, empty segments, null bytes
  - AC-5.4: Parameter substitution unit tests — via injected env lookup
  - AC-5.5: Property tests (proptest) for RemoteSpec parsing, config name validation, QemuConfig serialization round-trips
  - AC-5.6: Command-specific integration test matrix:
    - save: basic save, with description, force overwrite, missing qemu_bin arg
    - exec: config not found, debug mode flags, full mode output
    - edit: config not found, no changes made
    - list: empty dir, multiple configs
    - print: config not found, full output format
    - rm: config not found, successful delete
    - rename: source not found, target exists with force, desc update
    - pull: invalid remote ref, force overwrite
    - push: local config not found, invalid remote ref
    - completions: bash/zsh/fish shell generation
  - AC-5.7: Error path tests verify exit codes and stderr contain expected error category indicators (not exact text)
  - AC-5.8: All 30 existing tests pass unchanged

- AC-6: Security hardening (in scope per user decision)
  - Positive Tests (expected to PASS):
    - `validate_config_name("valid-name_1")` accepts alphanumeric + dash + underscore + dot
    - `validate_segment("id", "team")` accepts valid segments
  - Negative Tests (expected to FAIL):
    - `validate_config_name("../etc/passwd")` returns Err
    - `validate_config_name("name\x00evil")` returns Err (null byte)
    - `validate_config_name("")` returns Err
    - `validate_segment("id", "../../../../etc")` returns Err
    - Remote spec with excessively long segments (>255 chars) returns Err
  - AC-6.1: validate_config_name rejects path separators (/ \), null bytes, empty, ".", ".."
  - AC-6.2: Remote validate_segment hardened for null bytes and length limits
  - AC-6.3: edit_command $EDITOR treated as trusted local input (documented assumption, no code change)

- AC-7: Final audit gate
  - Positive Tests (expected to PASS):
    - `rg "anyhow" src/config/ src/commands/ src/remote/` returns zero matches
    - `cargo test` exits 0
    - CLI smoke test: save + list + print + rm round-trip succeeds
  - Negative Tests (expected to FAIL):
    - No `anyhow::bail!` in domain modules
    - No `.context(` in domain modules
  - AC-7.1: grep confirms no anyhow in config/, commands/, remote/
  - AC-7.2: cargo test passes all tests
  - AC-7.3: cargo fmt --check passes
  - AC-7.4: cargo clippy passes
  - AC-7.5: CLI smoke test round-trip works

## Path Boundaries

Path boundaries define the acceptable range of implementation quality and choices.

### Upper Bound (Maximum Acceptable Scope)

The implementation expands VexError to 13 variants with full `thiserror` derive, migrates all ~50 anyhow sites in domain layers, extracts config loading pipeline (parse/load/validate), extracts parameter substitution with injected env lookup, adds comprehensive security validation for config names and remote specs, and delivers a full test suite with unit tests for all pure functions, property tests for parsers/validators, and integration tests for all 10 commands with multiple paths each. The test suite uses proptest for combinatorial coverage of parsing and validation. All existing 30 tests continue to pass.

### Lower Bound (Minimum Acceptable Scope)

The implementation expands VexError to cover all error categories with at least the core variants, migrates config/ and commands/ away from anyhow (remote/ may use a simpler conversion), adds validate_config_name with basic checks (empty, path separators), extracts substitute_params with env closure, and delivers unit tests for error variants + config validation + at least one integration test per command covering the happy path.

### Allowed Choices

- Can use: `thiserror` crate for error derive macros (user-approved)
- Can use: `proptest` crate as dev-dependency for property-based tests
- Can use: existing `escargot` + `tempfile` pattern for integration tests
- Can use: `matches!` macro for error variant assertions in tests
- Cannot use: `anyhow` in domain modules (config/, commands/, remote/) after migration
- Cannot use: `unsafe { std::env::set_var() }` in tests (Rust 2024 edition unsafe)
- Cannot use: trait-based I/O abstraction (out of scope; keep direct fs calls)
- Cannot use: in-memory git backend (out of scope; keep real git for remote tests)

## Feasibility Hints and Suggestions

> **Note**: This section is for reference and understanding only. These are conceptual suggestions, not prescriptive requirements.

### Conceptual Approach

```
1. Expand VexError in src/error.rs using thiserror:

   #[derive(Debug, thiserror::Error)]
   pub enum VexError {
       #[error("Configuration '{name}' not found")]
       ConfigNotFound { name: String },
       
       #[error("Configuration '{name}' already exists")]
       ConfigAlreadyExists { name: String },
       
       #[error("Failed to parse configuration")]
       ConfigParseFailed { #[source] source: serde_json::Error },
       
       #[error("Failed to serialize configuration")]
       ConfigSerializeFailed { #[source] source: serde_json::Error },
       
       #[error("Validation failed: {reason}")]
       ValidationError { field: Option<String>, reason: String },
       
       #[error("IO error on {path}: {operation}")]
       IoError { path: PathBuf, operation: String, #[source] source: std::io::Error },
       
       #[error("Failed to launch QEMU: {binary}")]
       QemuLaunchFailed { binary: String, #[source] source: std::io::Error },
       
       #[error("QEMU exited with code {exit_code:?}: {binary}")]
       QemuExitError { binary: String, exit_code: Option<i32> },
       
       #[error("Remote registry not configured (set {env_var})")]
       RemoteNotConfigured { env_var: String },
       
       #[error("Invalid remote spec '{input}': {reason}")]
       RemoteSpecInvalid { input: String, reason: String },
       
       #[error("git {args} failed")]
       GitCommandFailed { args: String, stderr: String, stdout: String, exit_code: Option<i32> },
       
       #[error("Remote config '{id}/{name}:{tag}' not found")]
       RemoteConfigNotFound { id: String, name: String, tag: String },
       
       #[error("Editor '{editor}' failed")]
       EditorFailed { editor: String, exit_code: Option<i32> },
   }
   
   pub type VexResult<T> = Result<T, VexError>;

2. Migrate incrementally: config/ → commands/ → remote/
3. Add validate_config_name() in config/validation.rs
4. Split config loading: parse_config_json() + load_config()
5. Extract substitute_params with env closure parameter
6. Write tests bottom-up: error variants → validation → parsing → integration
```

### Relevant References

- `src/error.rs` — existing VexError enum (5 variants, From impls, Display impl) to expand
- `src/config/storage.rs` — config_dir() and config_file() with anyhow::Context (migration targets)
- `src/config/validation.rs` — minimal validate_config() to expand
- `src/config/types.rs` — QemuConfig struct definition (pure data)
- `src/commands/exec.rs` — substitute_params() pure function to extract with env closure
- `src/remote/mod.rs` — RemoteSpec::parse(), validate_segment(), run_git() (migration targets)
- `src/tests/exec_tests.rs` — existing substitute_params unit tests (env mutation pattern to fix)
- `src/tests/test_save.rs` — escargot integration test pattern to follow
- `src/remote/mod.rs` inline tests — existing pure-function test pattern for remote parsing

## Dependencies and Sequence

### Milestones

1. **Error Architecture Foundation**: Expand VexError, add thiserror, define VexResult
   - Define all 13 error variants with context fields
   - Implement From traits for io::Error (contextual) and serde_json::Error
   - Add VexResult<T> type alias

2. **Config Foundation**: Config name validation + loading pipeline split
   - Add validate_config_name() — must happen before command migration
   - Expand validate_config() with additional checks
   - Split into parse_config_json() (pure) and load_config() (I/O + parse)
   - Harden remote validate_segment() for null bytes and length

3. **Config Layer Migration**: Migrate config/storage.rs and config/validation.rs to VexResult

4. **Command Layer Migration**: Migrate all 10 command modules incrementally
   - Ensure save and edit call validate_config
   - Apply validate_config_name at all name entry points
   - Replace all anyhow::bail! and .context() with VexError variants

5. **Remote Layer Migration**: Migrate remote/mod.rs to VexResult
   - GitCommandFailed preserves stderr/stdout
   - RemoteSpec errors use RemoteSpecInvalid variant

6. **CLI Boundary and Extraction**: lib.rs conversion + parameter substitution
   - lib.rs run() wraps VexError → anyhow for CLI presentation
   - Extract substitute_params with env lookup closure
   - Update existing substitute_params callers

7. **Unit Test Suite**: Tests for all pure functions and error variants
   - VexError construction, Display, source() chains
   - Config validation (valid/invalid cases)
   - Remote spec parsing (valid/invalid/adversarial)
   - Parameter substitution with injected env

8. **Property Tests**: proptest for parsers and validators
   - RemoteSpec parsing: arbitrary strings → parse never panics
   - Config name validation: arbitrary strings → validates or rejects, never panics
   - QemuConfig serialization round-trips: serialize → deserialize == original

9. **Integration Test Expansion**: All commands × key paths
   - Follow command-specific matrix from AC-5.6
   - Error path tests verify exit codes and stderr categories
   - All 30 existing tests verified passing

10. **Audit Gate**: Final verification
    - grep for stale anyhow usage in domain modules
    - cargo test, cargo fmt --check, cargo clippy
    - CLI smoke test round-trip

Milestone 1 has no dependencies. Milestone 2 depends on 1. Milestone 3 depends on 2. Milestones 4 and 5 depend on 3 and can proceed in parallel. Milestone 6 depends on 4 and 5. Milestones 7-9 depend on 6 and can proceed in parallel. Milestone 10 depends on 7, 8, and 9.

## Task Breakdown

Each task must include exactly one routing tag:
- `coding`: implemented by Claude
- `analyze`: executed via Codex (`/humanize:ask-codex`)

| Task ID | Description | Target AC | Tag (`coding`/`analyze`) | Depends On |
|---------|-------------|-----------|----------------------------|------------|
| task1 | Expand VexError: 13 variants with thiserror, context fields, From traits, VexResult alias | AC-1 | coding | - |
| task2 | Add validate_config_name() + expand validate_config() with additional checks | AC-3.3, AC-3.4, AC-6.1 | coding | task1 |
| task3 | Split config loading: parse_config_json (pure), load_config (I/O+parse) | AC-3.1, AC-3.2 | coding | task1 |
| task4 | Harden remote validate_segment: null bytes, length limits | AC-6.2 | coding | task1 |
| task5 | Migrate config/storage.rs and config/validation.rs to VexResult | AC-2.1 | coding | task2, task3 |
| task6 | Migrate commands: save, exec, edit, list, print, rm, rename + add validate calls | AC-2.2, AC-3.5 | coding | task5 |
| task7 | Migrate commands: push, pull, completions | AC-2.2 | coding | task5 |
| task8 | Migrate remote/mod.rs with GitCommandFailed stderr/stdout capture | AC-2.3 | coding | task1, task4 |
| task9 | Extract substitute_params with env lookup closure + lib.rs boundary conversion | AC-4, AC-2.4 | coding | task6, task7, task8 |
| task10 | Unit tests: VexError variants — construction, Display, source chains | AC-5.1 | coding | task1 |
| task11 | Unit tests: config validation + name validation | AC-5.2 | coding | task2 |
| task12 | Unit tests: remote spec parsing + security hardening | AC-5.3 | coding | task4 |
| task13 | Unit tests: substitute_params with injected env lookup | AC-5.4 | coding | task9 |
| task14 | Property tests: RemoteSpec, config name, QemuConfig serde round-trips | AC-5.5 | coding | task11, task12 |
| task15 | Integration tests: all 10 commands per AC-5.6 matrix | AC-5.6 | coding | task9 |
| task16 | Error path integration tests: exit codes + stderr category verification | AC-5.7 | coding | task15 |
| task17 | Verify all 30 existing tests pass unchanged | AC-5.8 | coding | task9 |
| task18 | Audit: grep anyhow, cargo test, cargo fmt, cargo clippy, CLI smoke test | AC-7 | coding | task16, task17 |
| task19 | Post-migration audit for missing error categories | AC-1 | analyze | task9 |
| task20 | Audit edit_command sh -c invocation for injection surface | AC-6.3 | analyze | task6 |

## Claude-Codex Deliberation

### Agreements

- Typed VexError below the CLI presentation layer is the correct architectural direction
- Keeping anyhow only at lib.rs/main.rs CLI boundary is reasonable
- Migrating config/ first is sensible because it is shared by most commands
- Preserving underlying source errors (io::Error, serde_json::Error) via #[source] is required
- Pure test surfaces for remote parsing, config validation, name validation, and parameter substitution are worth extracting
- Config-name validation is a real security need because names affect filesystem paths
- Test architecture and behavior coverage matrix are more valuable than raw test counts
- Injecting env lookup into substitute_params fixes the unsafe set_var test problem in Rust 2024
- Final audit gate (grep + cargo test + CLI smoke) is concrete and useful
- utils/io.rs (2 prompt functions) is CLI glue and may keep anyhow

### Resolved Disagreements

- **Error taxonomy: StorageError vs IoError**: Claude proposed separate StorageError; Codex argued for consolidation. Resolution: merged into single IoError{path, operation, source} — simpler, no semantic overlap.

- **InvalidConfig vs ValidationError**: Claude proposed both; Codex flagged overlap. Resolution: merged into single ValidationError{field, reason} covering all semantic validation. ConfigNotFound and ConfigAlreadyExists remain separate (existence checks, not validation).

- **Serialization errors**: Claude initially omitted ConfigSerializeFailed in Round 3; Codex caught that serde_json::to_string_pretty() calls at 5 callsites need coverage. Resolution: added ConfigSerializeFailed{source} alongside ConfigParseFailed{source}.

- **1000+ test target**: Claude initially included as AC; Codex argued it's a vanity metric for 2,378 LOC. Resolution: user decided behavior-class coverage is the real goal. Test count is a consequence of thorough coverage, not a primary target.

- **$EDITOR injection**: Codex flagged that AC-6.3 understated the risk — the injection surface is $EDITOR (user-controlled), not the temp path. Resolution: documented as trusted local input (user controls their own shell). The temp path is system-controlled. No code change needed.

- **Integration test specificity**: Codex flagged AC-5.6 as too generic ("config-not-found doesn't apply to save/list/completions"). Resolution: replaced with command-specific test matrix listing exactly which paths apply to each command.

- **EditorFailed variant**: Codex suggested adding exit_code field. Resolution: accepted, EditorFailed{editor, exit_code: Option<i32>}.

### Convergence Status

- Final Status: `converged`
- Rounds: 3 (Round 1: 12 required changes → Round 2: 5 required changes → Round 3: 1 required change → converged)

## Pending User Decisions

- DEC-1: Use thiserror crate
  - Claude Position: Use thiserror for ergonomic derive macros
  - Codex Position: Acceptable; standard Rust ecosystem practice
  - Tradeoff Summary: Adds one dependency but significantly reduces boilerplate
  - Decision Status: **Use thiserror** (user approved)

- DEC-2: Keep anyhow at CLI boundary permanently or plan for full removal
  - Claude Position: Keep anyhow at CLI boundary for now; full removal is a separate future decision
  - Codex Position: Keep permanently until typed errors stabilize; removing it can be a later cleanup
  - Tradeoff Summary: Keeping anyhow at the boundary simplifies error presentation and avoids premature optimization of the CLI layer
  - Decision Status: **Keep for now, defer full removal** (both sides agree)

- DEC-3: CLI error message stability
  - Claude Position: Only error categories and exit codes are stable
  - Codex Position: Same — only categories and exit codes
  - Tradeoff Summary: Allows error messages to improve without breaking tests; tests verify categories via exit codes and stderr substring patterns
  - Decision Status: **Categories and exit codes stable only** (user approved)

- DEC-4: Test count target
  - Claude Position: Behavior-class coverage is the real goal
  - Codex Position: Same — behavior classes, not numeric targets
  - Tradeoff Summary: Coverage matrix ensures meaningful tests; proptest generates additional cases naturally
  - Decision Status: **Behavior coverage priority** (user approved)

- DEC-5: Security hardening scope
  - Claude Position: Include in this refactor since validation logic is tightly coupled to error types
  - Codex Position: Acceptable if it doesn't delay the core refactor
  - Tradeoff Summary: Security validation (config name, remote spec) naturally fits alongside error type migration
  - Decision Status: **In scope for this refactor** (user approved)

## Implementation Notes

### Code Style Requirements
- Implementation code and comments must NOT contain plan-specific terminology such as "AC-", "Milestone", "Step", "Phase", or similar workflow markers
- These terms are for plan documentation only, not for the resulting codebase
- Use descriptive, domain-appropriate naming in code instead
- Follow existing code conventions: Rust 2024 edition, 4-space indentation
- Add `thiserror` to `[dependencies]` and `proptest` to `[dev-dependencies]` in Cargo.toml
- Do not add `anyhow` imports in any domain module after migration
- Use `matches!` macro for error variant assertions in tests rather than implementing PartialEq on VexError

--- Original Design Draft Start ---

# Typed Error Architecture With Comprehensive Test Suite For Vex

## Original Idea

帮我重构和优化 vex 的代码实现，包括所有功能，并补充至少1000个单元测题

## Primary Direction: Typed Error Architecture

### Rationale

Replace anyhow with domain-specific error enums to make error paths explicit, testable, and self-documenting — distinct from other directions because it focuses specifically on the error type hierarchy.

### Approach Summary

Replace the generic `anyhow::Result<T>` error handling throughout vex with a comprehensive domain-specific error enum that clearly categorizes all failure modes. The core mechanism:

1. **Expand `VexError` enum**: Grow the existing 4-variant `VexError` in `src/error.rs` to 10-12 variants covering all 6 error categories discovered in the codebase: Configuration (not found, exists, invalid, storage), Serialization (JSON deserialize/serialize/invalid format), IO/Storage (read, write, mkdir, dir entries), Execution (QEMU launch, exit code), Remote Operations (parse, clone, git command, path conversion), and Validation (segment validation, binary path).

2. **Implement `From` traits**: Add conversion implementations for `std::io::Error`, `serde_json::Error`, and custom domain errors to enable seamless `?` operator usage across the codebase.

3. **Systematic migration**: Replace all 52 documented `anyhow` usage sites across: `src/config/` (6 sites in storage.rs, validation.rs), `src/commands/` (26 sites across exec, save, pull, push, edit, remove, rename, print, list), `src/remote/` (16 sites in mod.rs for git/parse/network operations), and `src/utils/` (2 sites in io.rs).

4. **Test infrastructure**: With explicit error variants, each error path becomes independently testable. Enable exhaustive pattern matching in tests to verify correct error propagation for each of the 52 error sites plus boundary conditions, targeting 1000+ unit tests across all error categories.

### Objective Evidence

- `src/error.rs` (lines 4-11) already contains a partial `VexError` enum with 4 variants (ConfigNotFound, ConfigAlreadyExists, InvalidConfig, IoError, SerializationError) — this is the architectural foundation to extend
- 52 documented `anyhow` usage sites distributed across: `src/config/` (6), `src/commands/` (26: exec 6, save 2, pull 2, push 2, edit 5, remove 1, rename 5, print 3, list 2), `src/remote/mod.rs` (16 sites at lines 30, 104, 119, 127, 139, 192, 196, 205, 222, 265, 291, 300, 302, 320, 343, 349), `src/utils/` (2)
- Error messages cluster into 6 coherent categories: Configuration (4 variants), Serialization (3 variants), IO/Storage (5 variants), Execution (2 variants), Remote Operations (6 variants), Validation (2 variants)
- `src/remote/mod.rs` `validate_segment()` (lines 190-211) and `RemoteSpec::parse()` (lines 27-48) demonstrate structured validation with `.context()` messages — strong precedent for enum-based errors
- Tests in `src/tests/test_remote.rs` (lines 382-430) use `.unwrap()` on parse operations, indicating a pain point for error testing that typed errors would resolve
- Total codebase is ~2,378 LOC in src/; error-handling code represents ~52 bail/context sites (2.2% of codebase)

### Known Risks

- Migration scope: 52 sites to refactor across 9 command modules requires systematic replacement to avoid incomplete conversions leaving anyhow alongside VexError
- Pattern consistency: Different command modules use `.context()` differently (some with formatters, some static strings); standardizing will require coordinating Display impls with context construction
- Backward compatibility: Current top-level `fn main() -> anyhow::Result<()>` and pub Result types in `lib.rs` expose anyhow; type aliases need careful semver planning
- Git error detail loss: `src/remote/mod.rs` captures raw git stderr/stdout in bail! strings (lines 320-329); moving to enum risks losing stderr detail unless a dedicated field is added
- Testing complexity: 1000+ unit tests for error paths require parametrized test generators; many errors are IO-bound requiring mocking or fixture architecture

## Alternative Directions Considered

### Alt-1: Pure Logic Extraction
- Gist: Separate filesystem I/O from pure business logic by creating a core logic layer operating on in-memory data structures, with thin I/O adapters wrapping filesystem interactions. Refactor commands to delegate through Pure Logic to Filesystem I/O, enabling filesystem-free unit testing.
- Objective Evidence:
  - `src/commands/save.rs` (lines 48-123) mixes user prompts, direct filesystem writes, and external process execution with no way to unit-test save logic without actual files
  - `src/commands/exec.rs` (lines 132-142) `substitute_params()` is already a pure function — existing precedent for this pattern
  - `src/remote/mod.rs` has embedded unit tests (lines 376-431) for pure functions like `RemoteSpec::parse()` and `validate_segment()`
  - `src/config/types.rs` (14 LOC) `QemuConfig` is pure data (Serialize/Deserialize), testable in isolation
  - Existing tests in `src/tests/` (735 LOC) all require tempfile setup and binary compilation — no pure logic unit tests exist
- Why not primary: Requires introducing trait-based I/O abstractions (ConfigStore, FileSystem) that have no precedent in this codebase, adding architectural complexity before establishing the error foundation that typed errors provide.

### Alt-2: Property-Based Test Generation
- Gist: Use proptest to generate combinatorial config permutations, achieving 1200-1500 test cases from compact generators. Define Arbitrary implementations for QemuConfig and command args, then write 8-10 property functions each exercising 100+ generated cases to cover serialization round-trips, validation invariants, and parameter substitution edge cases.
- Objective Evidence:
  - `src/config/types.rs` QemuConfig has 4 fields (qemu_bin, args, desc, qemu_version) — ideal candidates for arbitrary fuzz generation
  - `src/config/validation.rs` checks only `qemu_bin.is_empty()` with a comment indicating "Additional validation logic can be added here" — prime property test target
  - `src/commands/exec.rs` line 132 `substitute_params()` uses regex — canonical property test domain
  - Current test count: 30 explicit `#[test]` functions; property tests would 40x coverage from ~200-300 LOC of generators
  - No proptest/quickcheck in Cargo.toml — clean addition with no conflicts
- Why not primary: Property-based tests are most effective when the code under test has well-defined contracts; typed error enums and pure logic extraction would first establish those contracts, making proptest generators more targeted.

### Alt-3: Trait-Based Command Dispatch
- Gist: Define a `Command` trait with associated types for Args and Output, refactor the 10-case match statement in `src/lib.rs` (lines 20-42) into a centralized registry, and unify the repetitive I/O and error handling patterns duplicated across 800 LOC of command modules.
- Objective Evidence:
  - `src/lib.rs` lines 20-42 contains a 10-case match with inconsistent arg unpacking (0-5 parameters per command)
  - 8 commands duplicate config I/O patterns (`fs::read_to_string`, `serde_json::from_str`, `fs::write`)
  - `config_path.exists()` + `anyhow::bail!` appears in 6 commands
  - Clap's `#[derive(Subcommand)]` enum in `src/commands/mod.rs` (lines 25-56) already provides structured dispatch precedent
  - Total command logic: ~1,068 LOC across commands + lib.rs
- Why not primary: Primarily architectural polish rather than addressing a critical pain point; the codebase is small enough that the current match dispatch is maintainable, and the refactoring ROI is moderate compared to typed errors which directly enable testability.

### Alt-4: Repository Abstraction for Remote
- Gist: Introduce a `GitBackend` trait abstracting the 26 direct git shell-out invocations in `src/remote/mod.rs`, with a `ShellGitBackend` for production and `InMemoryGitBackend` for testing, enabling push/pull logic verification without real git repositories.
- Objective Evidence:
  - `src/remote/mod.rs` contains 26 git operation invocations across 5 helper functions: `run_git()` (line 312), `git_succeeds()` (line 332), `git_output()` (line 338), `git_status_is_clean()` (line 307)
  - `src/commands/push.rs` (61 LOC) and `src/commands/pull.rs` (50 LOC) call `publish_config()` and `clone_remote_repo()` without abstraction
  - `src/tests/test_remote.rs` (172 LOC) has only 2 integration tests using real git repos
  - No existing trait-based abstractions anywhere in the codebase
- Why not primary: No prior trait-based patterns exist in the codebase, and `clone_remote_repo()` returns `(TempDir, PathBuf)` requiring deeper API redesign beyond just introducing a trait; git semantics are complex enough that an in-memory simulation risks masking real failures.

### Alt-5: CLI Integration Test Framework
- Gist: Establish an assert_cmd-based integration test framework exercising the full vex binary at the process boundary, with parametrized suites covering all 9 commands, shared test fixtures (TempDir utilities, mock remote repos, config builders), and scaling to 1000+ tests via arg permutations and multi-command workflow scenarios.
- Objective Evidence:
  - 30 existing tests in `src/tests/` already use escargot + tempfile pattern — assert_cmd is a natural extension
  - `Cargo.toml` already declares `escargot = "0.5.15"` and `tempfile = "3.23.0"` as dependencies
  - 9 command implementations total ~916 LOC of testable command logic
  - `.github/workflows/ci.yml` line 37 runs `cargo test --verbose` — CI integration is automatic
  - Test count estimate: 300+ deterministic tests easily achievable; 1000+ via arg permutation and stress scenarios
- Why not primary: Integration tests verify behavior at the binary boundary but cannot efficiently test internal error paths or pure logic; typed error architecture enables finer-grained unit testing that integration tests complement but cannot replace.

## Synthesis Notes

The strongest implementation path combines the primary typed error architecture with elements from multiple alternatives. Property-based test generation (Alt-2) becomes dramatically more effective once typed errors define explicit contracts to test against — proptest generators can target each VexError variant systematically. Pure logic extraction (Alt-1) could proceed in parallel for the already-pure functions like `substitute_params()` and `RemoteSpec::parse()`, creating unit-testable islands without full I/O abstraction. The CLI integration test framework (Alt-5) provides the end-to-end safety net while unit and property tests cover internal logic. If the user prioritizes test count over refactoring depth, combining Alt-2 (proptest) with Alt-5 (assert_cmd integration) could achieve 1000+ tests faster, deferring the error architecture refactoring. Conversely, if code quality and maintainability are the priority, the primary direction plus Alt-1 provides the cleanest foundation that naturally generates testable surface area.

--- Original Design Draft End ---
