AGREE:
- Test inventory now matches the repo shape: 17 escargot + 8 substitute_params + 5 inline remote = 30.
- Keeping `utils/io.rs` as CLI glue with `anyhow` is reasonable.
- Moving config-name validation to M2 is correct; it must happen before command migration.
- Splitting parse/load/validate is the right direction.
- Injecting env lookup into `substitute_params` fixes the unsafe `set_var` test problem.
- Final audit gate is concrete and useful.

DISAGREE:
- Convergence is close, but not achieved yet.
- `SerializationError{path, source}` conflicts with `parse_config_json(content: &str)`, because the pure parser has no path.
- `InvalidConfig{name, reason}` and `ValidationError{field, reason}` still overlap. Also `validate_config(config)` cannot produce `InvalidConfig{name, ...}` without a name.
- AC-5.6 is still too generic: `config-not-found` does not apply cleanly to `save`, `list`, or `completions`; `push/pull` need remote-specific paths.
- AC-6.3 understates the injection surface. The risky part is shelling through user-controlled `$EDITOR`, not the temp path.

REQUIRED_CHANGES:
- Clarify parse/serialization errors: either make `parse_config_json` return a pathless parse error, or pass path/name context into it.
- Resolve `InvalidConfig` vs `ValidationError` taxonomy with exact ownership for each.
- Define whether `load_config` validates configs. If not, every consuming command must call `validate_config`.
- Replace AC-5.6 with a command-specific integration matrix.
- Fix AC-6.3 to explicitly decide: keep `sh -c` and document `$EDITOR` as trusted local input, or remove shell invocation and run the editor directly.

OPTIONAL_IMPROVEMENTS:
- Remove exact migration site counts from AC-2 or define what “site” means; current `rg` counts do not line up with 6/26/16.
- Add `cargo fmt --check` and `cargo clippy` to the audit gate.
- Make `proptest` an explicit dev-dependency decision.
- Split M9 into smaller batches so integration-test expansion does not become one large brittle task.

UNRESOLVED:
- DEC-1: `thiserror` vs manual `Display`/`Error`.
- DEC-2: keep `anyhow` permanently at CLI boundary vs future full removal.
- DEC-3: whether CLI stderr text is stable, or only categories/exit codes are stable.
- DEC-4: 1000+ tests vs behavior-class coverage.
- DEC-5: whether security hardening is in scope for this plan or separate.
