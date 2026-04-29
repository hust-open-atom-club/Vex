# Ask Codex Input

## Question

## Convergence Round 2: Revised Candidate Plan

Review this REVISED plan. Prior round's REQUIRED_CHANGES have been addressed. Check if convergence is achieved.

### Changes Made From Round 1:
1. FIXED: Test inventory corrected to 17 escargot + 8 substitute_params + 5 remote inline = 30 total
2. FIXED: Error taxonomy consolidated — no StorageError; IoError carries path+operation context
3. FIXED: utils/io.rs classified as CLI glue, keeps anyhow (2 prompt functions only)
4. FIXED: Config-name validation moved to M2 (before command migration)
5. FIXED: Config loading split into parse_config_json / load_config / validate_config
6. FIXED: save and edit now call validate_config consistently
7. FIXED: Test target is coverage-matrix-based; 1000+ is a user decision not an AC
8. FIXED: Parameter substitution extraction uses injected env lookup closure
9. FIXED: Security hardening moved to M2 (name validation) and M4 (remote spec hardening)
10. FIXED: Final audit gate added (rg anyhow in domain layers, cargo test, CLI smoke tests)
11. DEFERRED to user: thiserror adoption, anyhow permanent status, CLI message stability

### REVISED ACCEPTANCE CRITERIA:

AC-1: VexError expanded with rich context fields
  AC-1.1: Config errors: ConfigNotFound{name}, ConfigAlreadyExists{name}, InvalidConfig{name,reason}
  AC-1.2: IoError{path,operation,source:io::Error} — covers all filesystem operations
  AC-1.3: SerializationError{path,source:serde_json::Error}
  AC-1.4: ExecutionError: QemuLaunchFailed{binary,source:io::Error}, QemuExitError{binary,exit_code:Option<i32>}
  AC-1.5: RemoteError: RemoteNotConfigured, RemoteSpecInvalid{input,reason}, GitCommandFailed{args,stderr,stdout,exit_code:Option<i32>}, RemoteConfigNotFound{spec_display}
  AC-1.6: ValidationError{field,reason} — for semantic validation beyond deserialization
  AC-1.7: All variants implement Display with user-friendly messages and std::error::Error with proper source() chains

AC-2: Incremental migration with clear boundaries
  AC-2.1: config/ uses VexResult (6 sites, milestone M2)
  AC-2.2: commands/ uses VexResult (26 sites, milestone M4)
  AC-2.3: remote/ uses VexResult (16 sites, milestone M5)
  AC-2.4: lib.rs run() converts VexError → anyhow::Error at the CLI boundary
  AC-2.5: utils/io.rs remains anyhow (CLI glue, 2 sites)

AC-3: Config loading pipeline
  AC-3.1: parse_config_json(content:&str) → Result<QemuConfig, VexError> (pure, no I/O)
  AC-3.2: load_config(name:&str) → Result<QemuConfig, VexError> (I/O + parse)
  AC-3.3: validate_config(config:&QemuConfig) → Result<(), VexError> expanded checks
  AC-3.4: validate_config_name(name:&str) → Result<(), VexError> — rejects path separators, empty, too long, null bytes
  AC-3.5: save and edit both call validate_config after construction/editing

AC-4: Parameter substitution extraction
  AC-4.1: substitute_params takes env lookup closure: substitute_params(args, env_fn) where env_fn: Fn(&str)->Option<String>
  AC-4.2: Default production caller uses std::env::var
  AC-4.3: Tests use deterministic HashMap-based env lookup (no unsafe set_var)

AC-5: Test coverage matrix (not raw count)
  AC-5.1: Error variant unit tests — each VexError variant constructible, Display correct, source() chains work
  AC-5.2: Config validation unit tests — empty binary, invalid names, valid configs, boundary cases
  AC-5.3: Remote spec parsing unit tests — valid specs, path traversal, special chars, empty segments
  AC-5.4: Parameter substitution unit tests — defined/undefined vars, mixed, empty, edge cases (via injected env)
  AC-5.5: Property tests (proptest) for RemoteSpec parsing, config name validation, QemuConfig serialization round-trips
  AC-5.6: Integration tests per command — at least: happy path, config-not-found, invalid-args for each of 10 commands
  AC-5.7: Error path integration tests — verify exit codes and stderr contain expected error category indicators
  AC-5.8: All 30 existing tests pass unchanged

AC-6: Security hardening
  AC-6.1: validate_config_name rejects path separators (/ \), null bytes, empty, "." and ".."
  AC-6.2: Remote validate_segment already handles most — verify null byte and excessively long inputs
  AC-6.3: edit_command sh -c invocation audited for injection (temp path is controlled, but documented)

AC-7: Final audit gate
  AC-7.1: grep confirms no anyhow usage in config/, commands/, remote/ (except designated CLI glue)
  AC-7.2: cargo test passes all tests
  AC-7.3: CLI smoke test: save+list+print+rm+rename round-trip works

### REVISED MILESTONES:

M1: Error Architecture — expand VexError, add thiserror (if approved) or manual impls, VexResult alias
M2: Config Foundation — config-name validation, parse_config_json/load_config split, validate_config expansion
M3: Config Migration — migrate config/storage.rs, config/validation.rs to VexResult
M4: Command Migration — migrate all 10 commands incrementally, ensure save/edit call validate_config
M5: Remote Migration — migrate remote/mod.rs, GitCommandFailed with stderr/stdout capture
M6: CLI Boundary — lib.rs VexError → anyhow conversion, parameter substitution extraction
M7: Unit Test Suite — error variants, config validation, remote parsing, param substitution
M8: Property Tests — proptest for parsers, validators, serialization round-trips
M9: Integration Test Expansion — all commands × key paths, error path exit code verification
M10: Audit Gate — grep audit, cargo test, CLI smoke tests

### REVISED TASK BREAKDOWN:

| Task | Description | Target AC | Tag | Depends |
|------|------------|-----------|-----|---------|
| task1 | Expand VexError: all 6 categories with context fields, Display, Error, From | AC-1 | coding | - |
| task2 | Add validate_config_name + expand validate_config | AC-3.3,3.4,6.1 | coding | task1 |
| task3 | Split config loading: parse_config_json, load_config | AC-3.1,3.2 | coding | task1 |
| task4 | Migrate config/storage.rs and config/validation.rs | AC-2.1 | coding | task2,task3 |
| task5 | Migrate commands/ (save,exec,edit,list,print,rm,rename) + add validate_config calls | AC-2.2,3.5 | coding | task4 |
| task6 | Migrate commands/ (push,pull,completions) | AC-2.2 | coding | task4 |
| task7 | Migrate remote/mod.rs with GitCommandFailed variant | AC-2.3 | coding | task1 |
| task8 | Extract substitute_params with env closure + update lib.rs conversion | AC-4,2.4 | coding | task5,task6,task7 |
| task9 | Harden remote validate_segment (null bytes, length) | AC-6.2 | coding | task7 |
| task10 | Audit edit_command sh -c usage | AC-6.3 | analyze | task5 |
| task11 | Unit tests: VexError variants, Display, source chains | AC-5.1 | coding | task1 |
| task12 | Unit tests: config validation + name validation | AC-5.2 | coding | task2 |
| task13 | Unit tests: remote spec parsing + security | AC-5.3 | coding | task9 |
| task14 | Unit tests: substitute_params with injected env | AC-5.4 | coding | task8 |
| task15 | Property tests: RemoteSpec, config name, QemuConfig serde | AC-5.5 | coding | task12,task13 |
| task16 | Integration tests: all 10 commands × key paths | AC-5.6 | coding | task8 |
| task17 | Error path integration tests: exit codes + stderr | AC-5.7 | coding | task16 |
| task18 | Verify all 30 existing tests pass | AC-5.8 | coding | task8 |
| task19 | Final audit: grep anyhow, cargo test, CLI smoke | AC-7 | coding | task17,task18 |
| task20 | Audit for missing error categories post-migration | AC-1 | analyze | task8 |

### PENDING USER DECISIONS:
DEC-1: Add thiserror crate for ergonomic derive macros, or manual Display/Error impls?
DEC-2: Keep anyhow permanently at CLI boundary, or plan for full removal later?
DEC-3: Are CLI error messages a stable contract, or only error categories and exit codes?
DEC-4: Is 1000+ tests a hard requirement, or is behavior-class coverage sufficient?
DEC-5: Is security hardening (path traversal, injection) in scope or separate project?

---

Respond with exactly these sections:
AGREE: points accepted as reasonable
DISAGREE: points considered unreasonable and why
REQUIRED_CHANGES: must-fix items before convergence
OPTIONAL_IMPROVEMENTS: non-blocking improvements
UNRESOLVED: opposite opinions needing user decisions

## Configuration

- Model: gpt-5.5
- Effort: xhigh
- Timeout: 3600s
- Timestamp: 2026-04-29_22-27-22
- Tool: codex
