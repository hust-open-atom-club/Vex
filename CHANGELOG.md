# Changelog

All notable changes to Vex are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.2] - 2026-05-09

### Fixed
- `vex cache prune` no longer deletes cached objects that are referenced
  only by `resources[*].path` without a `sha256` (the case when remote
  configs publish url-only resources and `--fetch-resources` populates
  the cache without backfilling the hash).
- `vex pull --fetch-resources` and `vex hub install --fetch-resources`
  now backfill `sha256` and `size` on the downloaded resource so that
  cache reference tracking works without the path-fallback above.
- `vex hub list` no longer prefixes `latest_tag` with `v`, which produced
  invalid identifiers like `vlatest` or `vv1` that users could not pass
  back into `vex hub install`.

## [0.3.1] - 2026-05-09

### Fixed
- `vex hub info` and `vex hub install` now resolve omitted tags via
  `index.json`'s `latest_tag` field instead of assuming a `latest.json`
  alias, matching the protocol contract in `docs/HUB_PROTOCOL.md`.
- `vex hub install --as <NAME>` now validates the local name before
  constructing the config path, preventing path traversal via values
  like `../evil`.

## [0.3.0] - 2026-05-08

### Added
- Resource binding on `QemuConfig`: `ResourceRef { path, kind, sha256, size, url }` with `ResourceKind` (Image / Firmware / Other).
- `${res:KEY}` placeholder substitution in `vex exec`, replaced after `${ENV}` substitution; unknown keys raise `UnknownResourceReference`.
- `vex exec` now verifies that every bound resource file exists before launching QEMU.
- `vex save` flags `--image KEY=PATH`, `--firmware KEY=PATH`, `--resource KEY=PATH` (all repeatable), plus `--no-checksum` to skip the default sha256 + size capture.
- `vex resource` subcommand group: `add` (with `--kind`, `--no-checksum`, `--allow-missing`, `-f`), `rm`, `list`.
- `vex cache` subcommand group: `list`, `info <hash|prefix>`, `rm <hash> [-f]`, `prune [--dry-run]`. Cache layout is content-addressed at `<root>/<sha[0..2]>/<sha[2..]>`.
- `vex hub` subcommand group: `search <keyword>`, `info <id/name[:tag]>`, `install <spec> [--as NAME] [--fetch-resources] [--resource-dir DIR] [-f]`, `list [--kind KIND]`.
- `vex pull` flags `--fetch-resources` and `--resource-dir`, downloading referenced resources into the unified cache and rewriting local `path` to the cache location.
- Environment variable `VEX_RESOURCE_CACHE_DIR` overrides the default resource cache location.
- Environment variable `VEX_HUB_URL` selects the Vex Hub base URL (default: `https://hub.vex.example/`, placeholder).
- HTTP fetch layer: `remote::fetch::fetch_to_file` and `fetch_to_cache` (streaming, 30s timeout, content-addressed, sha256-verified when expected hash is known).
- Hub HTTP client (read-only) with `fetch_index` / `fetch_published_config`.
- `docs/HUB_PROTOCOL.md` defining the Hub HTTP protocol v1.
- Shell-completion overlays: cache-hash completion for `vex cache rm|info` and config-name completion for `vex resource add|rm|list`.
- 10 new `VexError` variants: `UnknownResourceReference`, `ResourceFileNotFound`, `ResourceChecksumMismatch`, `ResourceNotPublishable`, `ResourceFetchFailed`, `UnsupportedResourceScheme`, `SchemaVersionUnsupported`, `HubRequestFailed`, `HubIndexParseFailed`, `HubEntryNotFound`.

### Changed
- `PublishedConfig::new` produces `schema_version: 2`. `vex push` writes v2.
- `load_published_config` dispatches by `schema_version` (v2 / v1 / legacy bare `QemuConfig`); read-side never silently upgrades v1 to v2.
- `vex push` runs `validate_publishable` before clone; resources missing both `url` and `sha256` are rejected with `ResourceNotPublishable`.
- `vex pull` default cache directory now resolves through `resource_cache_dir()`, sharing the cache with `vex hub install`.

### Dependencies
- Added: `sha2` 0.10
- Added: `ureq` 2.10 (with `tls` feature)

## [0.2.0] - <unreleased>

### Added
- `vex push <id/name>[:tag] <local_name>` and `vex pull <id/name>[:tag]` for sharing configurations through a Git registry.
- `RemoteSpec` parser with id / name / tag segment validation.
- `PublishedConfig` schema v1 (`{ schema_version, id, name, tag, config }`), with `latest.json` auto-refresh on tagged push.
- Environment variables `VEX_REMOTE_URL`, `VEX_REMOTE_BRANCH`, `VEX_REMOTE_GIT_NAME`, `VEX_REMOTE_GIT_EMAIL`.
- Tolerant read path: fall back to plain `QemuConfig` JSON if the remote file predates the v1 envelope.

## [0.1.0] - <unreleased>

### Added
- Initial release: local QEMU configuration management.
- Commands: `save`, `rename`, `rm`, `list`, `print`, `exec`, `edit`, `completions`.
- `${ENV}` placeholder substitution for QEMU args at exec time.
- QEMU version capture on save and mismatch warning on exec.
- `VEX_CONFIG_DIR` environment variable; default `~/.vex/configs`.
- Bash / Zsh / Fish dynamic completion for saved configuration names.
