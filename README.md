# Vex

A Docker-like CLI for saving, sharing, and launching QEMU virtual machines.

Vex turns complex `qemu-system-*` invocations into named configurations you
can save, edit, distribute, and run by name. It targets embedded development,
firmware development, and OS development workflows where the same handful of
QEMU recipes get typed (and mistyped) every day.

![Browse mode](docs/screenshots/browse.png)

![Edit mode](docs/screenshots/edit-mode.png)

## Features

| Capability | Commands | Phase |
|---|---|---|
| Local config management | `save`, `list`, `print`, `rm`, `rename`, `edit`, `exec` | 1 |
| Shell completions | `completions` | 1 |
| Git remote distribution | `push`, `pull` | 2 |
| Resource binding | `save --image/--firmware/--resource`, `resource` | 3 |
| Resource cache | `cache` | 3 |
| Vex Hub (HTTP read-only client) | `hub` | 3 |
| Interactive TUI | `tui` | 4 |

## Installation

```bash
cargo build --release
```

Requires Rust 1.85+ (edition 2024).

## Quickstart

Local — save and run:

```bash
vex save my-vm qemu-system-x86_64 -m 2G -smp 4
vex list
vex exec my-vm
```

Team — push to a Git registry, pull elsewhere:

```bash
export VEX_REMOTE_URL=git@github.com:my-team/vex-registry.git
vex push team/dev-box:v1 dev-box      # publisher
vex pull team/dev-box:v1              # teammate
```

Public — install from a Vex Hub (see [Vex Hub](#vex-hub)):

```bash
export VEX_HUB_URL=https://hub.example.com/
vex hub install team/demo-arm64:v1 --fetch-resources
```

## Interactive TUI

```bash
vex tui
```

Two-pane Browse mode for navigating and launching saved configurations,
in-place `e` / `n` Edit mode with a Snippets drawer, and a full-screen
Library mode (`Ctrl+L`) for managing the user snippet library. Builtin
snippets are read-only; user overrides persist atomically to
`~/.vex/snippets.json`.

Full keybindings, workflows, and screenshots:
**[docs/TUI.md](docs/TUI.md)**.

## Resource binding

A configuration's `args` may reference external files (disk images, firmware
blobs, …) declared as **resources**. Resources are bound by key, validated
on save, and substituted into args at exec time using `${res:KEY}`.

```bash
vex save vm1 qemu-system-x86_64 \
    --image disk=./ubuntu.qcow2 \
    -m 2G \
    -drive 'file=${res:disk},format=qcow2'

vex resource list vm1
vex exec vm1
```

`${res:KEY}` placeholders only match keys matching `[A-Za-z_][A-Za-z0-9_]*`;
other forms are left as literal text. Unknown keys at `exec` time raise
`UnknownResourceReference`. By default, `vex save` captures sha256 + size;
add `--no-checksum` to skip when files are large or absent locally.

## Resource cache

`vex pull --fetch-resources` and `vex hub install --fetch-resources` write
downloaded files into a unified content-addressed cache at
`<config_dir parent>/resources` (default: `~/.vex/resources`).

```bash
vex cache list
vex cache info <hash-or-prefix>
vex cache rm <hash> [--force]
vex cache prune [--dry-run]
```

Hash arguments accept the full 64 hex characters or any prefix of at least
4 characters. Override the cache location with `VEX_RESOURCE_CACHE_DIR`.

## Vex Hub

The Vex Hub is a read-only HTTP service that distributes curated
configurations and resource metadata. It serves the same `PublishedConfig`
v2 format that `vex push` writes to a Git remote — Git remotes and the Hub
are complementary, not competing.

```bash
vex hub list --kind demo
vex hub search arm64
vex hub info team/demo-arm64:v1
vex hub install team/demo-arm64:v1 --as my-arm --fetch-resources
```

Protocol: **[docs/HUB_PROTOCOL.md](docs/HUB_PROTOCOL.md)**.

> **Note**: There is no officially deployed Hub server yet. The default
> `VEX_HUB_URL` (`hub.vex.example`) uses the RFC 2606 reserved TLD and is
> intentionally non-resolvable. To use `vex hub`, point `VEX_HUB_URL` at
> a server that implements the protocol above.

## Environment variables

| Variable | Purpose | Default |
|---|---|---|
| `VEX_CONFIG_DIR` | Local config storage directory | `~/.vex/configs` |
| `VEX_REMOTE_URL` | Git remote registry URL or local path | (none, push/pull errors out) |
| `VEX_REMOTE_BRANCH` | Branch used for remote distribution | `main` |
| `VEX_REMOTE_GIT_NAME` | Git author name for `vex push` commits | `Vex CLI` |
| `VEX_REMOTE_GIT_EMAIL` | Git author email for `vex push` commits | `vex@example.invalid` |
| `VEX_RESOURCE_CACHE_DIR` | Resource cache directory | `<config_dir parent>/resources` |
| `VEX_HUB_URL` | Vex Hub base URL | `https://hub.vex.example/` (placeholder) |

## Configuration file format

Each saved configuration is a JSON file at `<VEX_CONFIG_DIR>/<name>.json`.
The schema is the `QemuConfig` struct in
[`src/config/types.rs`](src/config/types.rs):

```json
{
  "qemu_bin": "qemu-system-x86_64",
  "args": ["-m", "2G", "-drive", "file=${res:disk}"],
  "desc": "Ubuntu development VM",
  "qemu_version": "9.0.1",
  "resources": {
    "disk": {
      "path": "/var/cache/vex/aa/bbbb...",
      "kind": "image",
      "sha256": "aabb...",
      "size": 1073741824,
      "url": "https://example.com/ubuntu.qcow2"
    }
  }
}
```

`desc`, `qemu_version`, `resources`, and the optional fields inside each
`ResourceRef` are omitted from output when unset.

## Documentation

- **[docs/TUI.md](docs/TUI.md)** — Interactive TUI manual (keybindings,
  workflows, screenshots).
- **[docs/HUB_PROTOCOL.md](docs/HUB_PROTOCOL.md)** — Hub HTTP protocol spec.
- **[CHANGELOG.md](CHANGELOG.md)** — Release history.

## Roadmap

- ✅ **Phase 1** (0.1.0): Local config management.
- ✅ **Phase 2** (0.2.0): Git-remote distribution, `PublishedConfig` v1.
- ✅ **Phase 3** (0.3.0): Resource binding, content-addressed cache,
  Vex Hub HTTP client, `PublishedConfig` v2.
- ✅ **Phase 4** (0.4.x): Interactive TUI — Browse, Edit, Snippets drawer,
  Library mode.
- **Future**: reference Hub server, web UI, resource signing, plugins.

## License

See [LICENSE](LICENSE).
