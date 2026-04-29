# Ask Codex Input

## Question

## Candidate Plan Review Request

Review this candidate plan for the Vex CLI refactoring project. The plan was built from a draft + first Codex analysis.

### CANDIDATE PLAN v1

**Goal**: Systematically refactor the vex codebase (~2,378 LOC Rust CLI) to replace all anyhow usage in domain layers with typed VexError variants, extract pure logic functions for filesystem-free testing, and build a comprehensive test suite combining unit tests, property-based tests, and integration tests covering all 10 commands and all error paths.

**Current State**:
- VexError enum exists in src/error.rs with 5 variants (ConfigNotFound, ConfigAlreadyExists, InvalidConfig, IoError, SerializationError) + From impls — BUT it is NOT USED anywhere in the codebase
- All 52 error sites use anyhow::Result, anyhow::bail!, and .context()
- 30 existing tests: 22 integration tests (escargot+tempfile binary invocation) + 8 unit tests (substitute_params)
- Pure functions already exist: substitute_params(), RemoteSpec::parse(), validate_segment(), normalize_remote_url(), is_explicit_git_url(), is_scp_style_git_url()

**Acceptance Criteria**:

AC-1: VexError enum expanded to cover all 6 error categories with #[source] preservation
  AC-1.1: Configuration errors (ConfigNotFound, ConfigAlreadyExists, InvalidConfig, StorageError with path context)
  AC-1.2: Serialization errors wrapping serde_json::Error with config name/path context
  AC-1.3: IO errors wrapping std::io::Error with operation+path context
  AC-1.4: Execution errors (QemuLaunchFailed, QemuExitError with exit code)
  AC-1.5: Remote errors (RemoteNotConfigured, RemoteSpecInvalid, GitCommandFailed with stderr, RemoteConfigNotFound)
  AC-1.6: Validation errors with field-level detail

AC-2: Incremental migration — domain modules use VexError
  AC-2.1: src/config/ migrated first (6 anyhow sites)
  AC-2.2: src/commands/ migrated second (26 sites)
  AC-2.3: src/remote/ migrated third (16 sites)
  AC-2.4: Top-level lib.rs keeps anyhow for CLI presentation glue (converts VexError → anyhow::Error)

AC-3: Pure logic extraction
  AC-3.1: Config validation expanded (empty binary, invalid names, malformed JSON, missing required fields)
  AC-3.2: Remote spec parsing/validation fully testable without git
  AC-3.3: Parameter substitution fully covered including edge cases

AC-4: Comprehensive test suite
  AC-4.1: Unit tests for all pure logic functions (~50-80 tests)
  AC-4.2: Property-based tests using proptest for RemoteSpec parsing, config validation, parameter substitution, QemuConfig serialization round-trips (~8-10 property functions × 100+ cases each = 800-1000 generated cases)
  AC-4.3: Integration tests for all 10 commands, happy path + error paths (~60-80 tests)
  AC-4.4: Error path tests verifying correct VexError variants are returned (~40-60 tests)
  AC-4.5: Total target: 1000+ test cases (combining explicit #[test] functions + proptest-generated cases)

AC-5: Behavioral preservation
  AC-5.1: All existing 30 integration tests pass unchanged
  AC-5.2: Exit code behavior documented and tested
  AC-5.3: Error messages may change wording but remain user-friendly and informative

AC-6: Validation and security hardening
  AC-6.1: Malformed JSON config files handled gracefully (not panics)
  AC-6.2: Remote spec rejects path traversal (../, ..\), shell metacharacters, null bytes
  AC-6.3: Config names validated (no path separators, no empty names, no excessively long names)

**Milestones**:

M1: Error Architecture (expand VexError, implement Display, From traits, define VexResult type alias)
M2: Config Layer Migration (migrate config/storage.rs, config/validation.rs to VexResult)
M3: Command Layer Migration (migrate all 10 commands incrementally)
M4: Remote Layer Migration (migrate remote/mod.rs, keep git error detail in GitCommandFailed variant)
M5: Pure Logic Extraction (expand validate_config, add config name validation, extract more pure functions)
M6: Unit Test Suite (tests for all pure functions + all error variants)
M7: Property-Based Tests (proptest for parsers, validators, serialization)
M8: Integration Test Expansion (cover all commands × all significant paths)
M9: Security Hardening (remote spec injection, config name validation, path traversal prevention)

**Task Breakdown**:

| Task | Description | Target AC | Tag | Depends |
|------|------------|-----------|-----|---------|
| task1 | Expand VexError enum with all 6 categories, Display, From traits | AC-1 | coding | - |
| task2 | Add VexResult type alias, update config/ to use VexResult | AC-2.1 | coding | task1 |
| task3 | Migrate commands/ to VexResult (incremental per command) | AC-2.2 | coding | task2 |
| task4 | Migrate remote/mod.rs to VexResult with GitCommandFailed | AC-2.3 | coding | task1 |
| task5 | Update lib.rs conversion layer (VexError → anyhow for CLI) | AC-2.4 | coding | task2,task3,task4 |
| task6 | Expand validate_config + add config name validation | AC-3.1, AC-6.3 | coding | task2 |
| task7 | Add remote spec security validation | AC-6.2 | coding | task4 |
| task8 | Unit tests for VexError variants and From impls | AC-4.1 | coding | task1 |
| task9 | Unit tests for config validation functions | AC-4.1 | coding | task6 |
| task10 | Unit tests for remote spec parsing + security | AC-4.1 | coding | task7 |
| task11 | Property tests: RemoteSpec, QemuConfig, substitute_params | AC-4.2 | coding | task6,task7 |
| task12 | Integration tests for all 10 commands | AC-4.3 | coding | task5 |
| task13 | Error path integration tests | AC-4.4 | coding | task5 |
| task14 | Verify all existing tests pass | AC-5.1 | coding | task5 |
| task15 | Audit for missing error categories | AC-1 | analyze | task5 |

**Questions / Pending Decisions**:
1. Is 1000+ tests a hard requirement, or is "comprehensive coverage across all behavior classes" the real goal?
2. Should anyhow be fully removed from Cargo.toml or kept for top-level CLI presentation?
3. Should error messages remain stable across versions (user-facing contract) or can they change freely?
4. Should VexError implement PartialEq for easier test assertions?

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
- Timestamp: 2026-04-29_22-22-09
- Tool: codex
