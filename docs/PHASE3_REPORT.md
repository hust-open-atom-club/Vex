# Phase 3 Implementation Report

## Summary

Phase 3 of Vex is complete and tagged as version 0.3.0. The release adds
resource binding, a content-addressed resource cache, and a read-only HTTP
Hub client. The on-disk and on-wire formats remain backward-compatible with
0.2.x (Git registry payloads) and pre-0.2 (legacy bare `QemuConfig` JSON).

## Deliverables

### New commands

- `vex resource {add, rm, list}` — manage resource bindings on an existing
  configuration.
- `vex cache {list, info, rm, prune}` — inspect and maintain the
  content-addressed resource cache.
- `vex hub {search, info, install, list}` — read-only HTTP client for the
  Vex Hub protocol.

### Extended commands

- `vex save` accepts `--image KEY=PATH`, `--firmware KEY=PATH`,
  `--resource KEY=PATH` (each repeatable), and `--no-checksum`.
- `vex pull` accepts `--fetch-resources` and `--resource-dir DIR`.
- `vex exec` substitutes `${res:KEY}` placeholders and verifies bound
  resource files exist before launching QEMU.

### New environment variables

- `VEX_RESOURCE_CACHE_DIR` — overrides the default resource cache directory
  (`<config_dir parent>/resources`).
- `VEX_HUB_URL` — Vex Hub base URL; defaults to `https://hub.vex.example/`
  (RFC 2606 reserved TLD, intentionally non-resolvable).

### New / modified files

New:

- `src/hub/{mod.rs,types.rs,fetch.rs}` — Hub client.
- `src/commands/resource.rs`, `src/commands/cache.rs`, `src/commands/hub.rs`
  — command implementations.
- `src/utils/hash.rs` — sha256 helpers.
- `src/remote/fetch.rs` — HTTP fetch + content-addressed cache writer.
- `src/tests/test_resource.rs`, `src/tests/test_cache.rs`,
  `src/tests/test_hub.rs` — integration suites.
- `docs/HUB_PROTOCOL.md` — Hub HTTP protocol v1 spec.
- `CHANGELOG.md` — Keep a Changelog 1.1.0 format.

Modified:

- `Cargo.toml` — version `0.1.0` → `0.3.0`; new deps `sha2`, `ureq`.
- `src/config/types.rs` — `QemuConfig.resources`, `ResourceRef`,
  `ResourceKind`.
- `src/config/validation.rs` — resource key/path/sha256/url validation;
  `validate_resource_key` exposed for reuse.
- `src/config/storage.rs` — `resource_cache_dir()`.
- `src/error.rs` — 10 new variants (see below).
- `src/lib.rs` — crate-level rustdoc, `pub mod hub`, dispatch arms.
- `src/commands/mod.rs` — registration, `after_help` env-var listing.
- `src/commands/save.rs` — `--image/--firmware/--resource/--no-checksum`,
  `build_resources` helper.
- `src/commands/exec.rs` — `substitute_resources`, `check_resource_files`,
  `Resources` block in `--full` startup banner.
- `src/commands/pull.rs` — `--fetch-resources`, `--resource-dir`,
  rewriting resource paths to cache locations on success.
- `src/commands/completions.rs` — bash/zsh/fish overlays for resource
  config-name and cache-hash completion.
- `src/remote/mod.rs` — `PublishedConfig::new` produces v2;
  `load_published_config` dispatches by `schema_version`;
  `validate_publishable` gate before clone in `publish_config`.
- `README.md` — full rewrite (env vars, quickstarts, resource binding,
  cache, Hub, roadmap).

### New error variants (10)

1. `UnknownResourceReference`
2. `ResourceFileNotFound`
3. `ResourceChecksumMismatch`
4. `ResourceNotPublishable`
5. `ResourceFetchFailed`
6. `UnsupportedResourceScheme`
7. `SchemaVersionUnsupported`
8. `HubRequestFailed`
9. `HubIndexParseFailed`
10. `HubEntryNotFound`

### Tests

- Phase 3 start: 233 tests
- Phase 3 end:   299 tests
- Net delta:    +66

Distribution of additions:

- `test_validation.rs` / `test_serde.rs` for `ResourceRef` schema:  +14
- `utils::hash::tests` (sha256 vectors + file streaming):           +3
- `exec_tests.rs` (placeholder substitution, resource file checks): +8
- `test_exec_integration.rs` (end-to-end exec with resources):      +2
- `test_save.rs` (new resource flags):                              +4
- `test_resource.rs` (resource subcommand group):                   +6
- `remote::fetch::tests` (HTTP server-backed fetch tests):          +4
- `test_push_pull.rs` (v2 push, v1 compat, --fetch-resources):      +4
- `test_serde.rs` (PublishedConfig v1/v2):                          +2
- `test_cache.rs` (cache subcommand group):                         +11
- `test_hub.rs` (hub subcommand group):                             +8
- (1 doctest stub on `vex::run` is a compile-only check, excluded.)

### Dependency growth

- Added: `sha2` 0.10
- Added: `ureq` 2.10 (with `tls` feature)

These are the only two new dependencies added in Phase 3.

## Backward compatibility

- **0.2.x clients pulling 0.3.x-published configs**: old clients will fail
  to deserialize `PublishedConfig` v2 because the embedded `QemuConfig`
  carries an unknown `resources` field (0.2.x's `QemuConfig` did not have
  it, and serde will error on unknown fields by default unless the type
  ships `#[serde(deny_unknown_fields)]` *off* — which it does, so this is
  *partially* recoverable: a 0.2.x client with default serde settings will
  actually accept the v2 payload, ignore the `resources` field, and behave
  as if no resources were defined. This was an unintended degree of
  forward compatibility that we will not rely on but also will not break.).
  In practice we recommend bumping clients to 0.3.x when consuming 0.3.x
  registries.
- **0.3.x clients pulling 0.2.x-published configs**: fully supported via
  the v1 branch in `load_published_config`. `schema_version` stays `1` on
  the read side; clients do not silently upgrade.
- **0.3.x clients reading legacy bare `QemuConfig` JSON (pre-0.2.0)**:
  fully supported via the legacy branch; the in-memory `PublishedConfig`
  carries `schema_version: 1` for these.

## Schema versions

| Schema | Producer | Consumer | Notes |
|---|---|---|---|
| Legacy bare `QemuConfig` | pre-0.2.0 | any | Wrapped at read time; in-memory `schema_version` is `1` |
| `PublishedConfig` v1 | 0.2.0 | 0.2.0+ | `resources` field absent or empty; `schema_version: 1` |
| `PublishedConfig` v2 | 0.3.0 | 0.3.0+ | `resources` may carry `url`, `sha256`, `size`; `schema_version: 2` |
| `HubIndex` v1 | (Phase 4) | 0.3.0+ | Hub server protocol; independent version namespace from `PublishedConfig` |

## What's NOT in 0.3.0

- A reference Hub server implementation. `docs/HUB_PROTOCOL.md` defines
  the wire format; no server is shipped in this repo.
- `oci://` resource downloads. `UnsupportedResourceScheme` is raised when
  encountered.
- Resource signing or attestation beyond sha256 content hashing.
- Pre-flight HEAD checks of resource URLs at `vex push` time.
- Parallel resource downloads. `vex pull --fetch-resources` and
  `vex hub install --fetch-resources` fetch sequentially by design.
- CI / release automation (no GitHub Actions / GitLab CI configs).
- Publication to crates.io.

## Known limitations

- `sha256` integrity is **not** verified by `vex exec` on the default
  fast path. The hash is captured at save / pull time and re-verified
  only when `vex pull --fetch-resources` downloads a fresh copy. Adding
  a `--verify` flag to exec is left for a future minor release.
- `vex hub install` without `--fetch-resources` records the original
  remote URLs but does not pre-populate the cache; users see a one-line
  "available at: …" notice for each resource and must re-run with
  `--fetch-resources` to materialize them.
- The Hub HTTP client does not implement `ETag` / `If-None-Match`. Every
  `vex hub list` / `search` re-downloads `index.json`. Server-side
  short-TTL `Cache-Control` is the only mitigation in v1.
- The default `VEX_HUB_URL` (`https://hub.vex.example/`) is a placeholder
  using an RFC 2606 reserved TLD; users must set this variable to a real
  Hub before any `vex hub` subcommand will succeed.
