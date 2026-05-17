# Vex

Vex is a QEMU auxiliary command-line tool that addresses three core pain points:
simplifying complex QEMU startup parameters, lowering the learning and usage
barrier for QEMU, and supporting remote distribution of configuration files.

It provides a Docker-like convenient experience, helping users quickly launch
full-system simulation environments — suitable for embedded development,
firmware development, operating system development, and similar scenarios.

## Features

| Capability | Commands | Phase |
|---|---|---|
| Local config management | `save`, `list`, `print`, `rm`, `rename`, `edit`, `exec` | 1 |
| Shell completions | `completions` | 1 |
| Git remote distribution | `push`, `pull` | 2 |
| Resource binding | `save --image/--firmware/--resource`, `resource` | 3 |
| Resource cache | `cache` | 3 |
| Vex Hub (HTTP read-only) | `hub` | 3 |
| Interactive TUI | `tui` | 4 |

## Interactive TUI

Vex includes an interactive terminal UI for browsing and launching saved
configurations:

    vex tui

Browse mode shows your configurations in the left pane and the selected
configuration's details on the right. Use j/k or arrow keys to navigate,
press Enter to launch the selected configuration, and q or Esc to quit.

### Key bindings (Browse mode)

| Key       | Action                                |
|-----------|---------------------------------------|
| j / ↓     | Move selection down                   |
| k / ↑     | Move selection up                     |
| g         | Jump to first configuration           |
| G         | Jump to last configuration            |
| Tab       | Switch focus between list and details |
| Enter     | Launch selected configuration in QEMU |
| /         | Enter filter mode                     |
| r         | Reload configurations from disk       |
| ?         | Toggle help overlay                   |
| q / Esc   | Quit TUI                              |
| Ctrl+C    | Force quit                            |

### Filter mode

Press `/` to filter the list by name or description. Type to refine,
Enter to accept (keep filter active), or Esc to clear.

### Requirements

- A terminal at least 100 columns wide is recommended.
- The TUI uses the alternate screen and raw mode; it cleanly restores
  the terminal on exit, error, or panic.

### Screenshots

(Screenshots will be added before the GitHub release: browse mode,
filter mode, and help overlay.)

## Environment Variables

| Variable | Purpose | Default |
|---|---|---|
| `VEX_CONFIG_DIR` | Local config storage directory | `~/.vex/configs` |
| `VEX_REMOTE_URL` | Git remote registry URL or local path | (none, push/pull errors out) |
| `VEX_REMOTE_BRANCH` | Branch used for remote distribution | `main` |
| `VEX_REMOTE_GIT_NAME` | Git author name for `vex push` commits | `Vex CLI` |
| `VEX_REMOTE_GIT_EMAIL` | Git author email for `vex push` commits | `vex@example.invalid` |
| `VEX_RESOURCE_CACHE_DIR` | Resource cache directory | `<config_dir parent>/resources` |
| `VEX_HUB_URL` | Vex Hub base URL | `https://hub.vex.example/` (placeholder) |

## Quickstart

### 1. Local: save and run

```bash
vex save my-vm qemu-system-x86_64 -m 2G -smp 4
vex list
vex exec my-vm
```

### 2. Team: push to a Git registry, pull on another machine

```bash
# On the publishing machine:
export VEX_REMOTE_URL=git@github.com:my-team/vex-registry.git
vex save dev-box qemu-system-aarch64 -m 4G
vex push team/dev-box:v1 dev-box

# On a teammate's machine:
export VEX_REMOTE_URL=git@github.com:my-team/vex-registry.git
vex pull team/dev-box:v1
vex exec dev-box
```

### 3. Public: install from Vex Hub

```bash
export VEX_HUB_URL=https://hub.example.com/   # your Hub server
vex hub search arm64
vex hub install team/demo-arm64:v1 --fetch-resources
vex exec demo-arm64
```

> **Note**: The Hub server is not yet officially deployed. The default
> `VEX_HUB_URL` (`hub.vex.example`) uses the RFC 2606 reserved TLD and is
> intentionally non-resolvable. To use `vex hub`, run your own server that
> implements [`docs/HUB_PROTOCOL.md`](docs/HUB_PROTOCOL.md) and point
> `VEX_HUB_URL` at it.

## Resource Binding

A configuration's `args` may reference external files (disk images, firmware
blobs, etc.) declared as **resources**. Resources are bound by key, validated
on save, and substituted into args at exec time using `${res:KEY}` placeholders.

```bash
# Bind a disk image at save time. Default behavior captures sha256 + size.
vex save vm1 qemu-system-x86_64 \
    --image disk=./ubuntu.qcow2 \
    -m 2G \
    -drive 'file=${res:disk},format=qcow2'

# Manage bindings on an existing config.
vex resource add vm1 bios /usr/share/firmware/edk2.fd --kind firmware
vex resource list vm1
vex resource rm vm1 bios

# Skip checksum capture when files are large or missing locally.
vex save vm2 qemu-system-arm \
    --no-checksum --image disk=/data/big.img

# Run — Vex verifies every bound file exists, then substitutes paths.
vex exec vm1
```

`${res:KEY}` placeholders only match keys conforming to
`[A-Za-z_][A-Za-z0-9_]*`. Other forms (e.g. `${res:my-disk}`) are left as
literal text. Unknown keys at exec time raise an `UnknownResourceReference`
error.

## Resource Cache

When `vex pull --fetch-resources` or `vex hub install --fetch-resources`
downloads files referenced by `ResourceRef.url`, they land in a unified
content-addressed cache.

- **Default location**: `<config_dir parent>/resources` (i.e. `~/.vex/resources`
  for the default `VEX_CONFIG_DIR`).
- **Override**: `export VEX_RESOURCE_CACHE_DIR=/path/to/cache`.
- **Layout**: `<root>/<sha256[0..2]>/<sha256[2..]>` — every object is named
  by its content hash, so multiple configurations referencing the same file
  share storage automatically.
- **Maintenance**:

  ```bash
  vex cache list                      # full listing with reference counts
  vex cache info <hash-or-prefix>     # details of one object
  vex cache rm <hash> [--force]       # delete (force required if referenced)
  vex cache prune [--dry-run]         # delete every unreferenced object
  ```

Hash arguments accept either the full 64 hex characters or any unique prefix
of at least 4 characters.

## Vex Hub

The Vex Hub is a read-only HTTP service that distributes curated configurations
and resource metadata. It serves the same `PublishedConfig` v2 format that
`vex push` writes to a Git remote, just over HTTP — Git remotes and the Hub
are complementary, not competing.

```bash
vex hub list --kind demo            # browse the catalog, optionally filtered
vex hub search arm64                # substring match on id / name / summary
vex hub info team/demo-arm64:v1     # show details of one entry
vex hub install team/demo-arm64:v1 --as my-arm --fetch-resources
```

Protocol specification: [`docs/HUB_PROTOCOL.md`](docs/HUB_PROTOCOL.md).

## Roadmap

### Phase 1: Local command capabilities ✅ 0.1.0
- `save`, `rename`, `rm`, `list`, `print`, `exec`, `edit`, `completions`.

### Phase 2: Remote distribution ✅ 0.2.0
- `push`, `pull` over a Git registry; `PublishedConfig` schema v1.

### Phase 3: Resource binding + Vex Hub ✅ 0.3.0
- Resource binding (`${res:KEY}`, `--image/--firmware/--resource`, `vex resource`).
- Content-addressed resource cache (`vex cache`, `VEX_RESOURCE_CACHE_DIR`).
- Vex Hub HTTP client (`vex hub`, `VEX_HUB_URL`, `docs/HUB_PROTOCOL.md`).
- `PublishedConfig` schema v2 with `url` / `sha256` / `size` per resource.

### Phase 4: Vex Hub Server (planned)
- Reference Hub server implementation following `docs/HUB_PROTOCOL.md`.
- Web UI for browsing entries.
- Resource signing and signature verification.
- Optional: pagination, server-side search backend, multi-tenant namespaces.

## Configuration File Format

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

## Building from Source

```bash
cargo build --release
```

Requires Rust 1.85+ (edition 2024).

## License

See [LICENSE](LICENSE).
