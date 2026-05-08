# Vex Hub Protocol v1

## Overview

The Vex Hub is a **read-only HTTP service** that distributes curated QEMU
configurations and resource metadata. It overlaps in purpose with the Git
remote registry used by `vex push` / `vex pull` (configured via
`VEX_REMOTE_URL`), but the transport layer is independent: the Hub speaks
plain HTTP and the on-disk payload format is `PublishedConfig` v2 — exactly
what the Git remote stores.

A Hub deployment is just a static file tree behind any HTTP server (nginx,
S3 + CloudFront, GitHub Pages, etc.). There is no write API in v1.

The client base URL is taken from `VEX_HUB_URL` (falling back to
`https://hub.vex.example/` when the variable is unset). All Hub-related
features are exposed through the `vex hub` subcommand group.

## Endpoints

### `GET /index.json`

Returns the catalog of entries the Hub publishes.

```json
{
  "schema_version": 1,
  "entries": [
    {
      "id": "team",
      "name": "demo-arm64",
      "latest_tag": "v2",
      "tags": ["v1", "v2", "latest"],
      "summary": "ARM64 demo board for evaluation",
      "kind": "demo",
      "updated_at": "2026-01-15T08:30:00Z"
    }
  ]
}
```

Fields:

- `schema_version: u32` — index format version, currently always `1`.
  This is **independent** of the `PublishedConfig.schema_version` (which is
  `2`) — the index can evolve without forcing a config-format bump.
- `entries: HubEntry[]`

`HubEntry` fields:

- `id: string` — namespace owner (e.g. team / org / person handle).
- `name: string` — entry name within the namespace.
- `latest_tag: string` — the tag the Hub considers "current" for this entry.
- `tags: string[]` — all tags currently published. May include `latest_tag`
  and `"latest"` itself.
- `summary: string` — short human-readable description.
- `kind: "demo" | "board" | "firmware" | "other"` — coarse category for
  filtering and display.
- `updated_at: string` — ISO 8601 timestamp. Clients are not required to
  parse this; v1 only displays it.

### `GET /configs/{id}/{name}/{tag}.json`

Returns a full `PublishedConfig` v2 document for a single entry+tag, with
`schema_version: 2` and an embedded `QemuConfig` (including `resources`
metadata such as `url`, `sha256`, `size`).

The wire format is byte-for-byte the same JSON shape that `vex push` writes
into the Git remote, and that `vex pull` reads back. Clients can therefore
share deserialization code between the Git and Hub paths.

`404 Not Found` is returned when the `(id, name, tag)` triple does not
exist.

## Errors

- `4xx` / `5xx` responses are surfaced to the user as
  `VexError::HubRequestFailed { url, status, body }`. The body is included
  verbatim so server-side error messages reach the user.
- Malformed `index.json` triggers `VexError::HubIndexParseFailed`.
- A successful 404 on `/configs/.../{tag}.json` is mapped to
  `VexError::HubEntryNotFound { id, name, tag }`.

## Caching

- The server **may** set short-TTL `Cache-Control` headers (e.g.
  `max-age=60`) to relieve origin pressure. This is a recommendation, not a
  requirement.
- The v1 client does not implement `ETag` / `If-None-Match`. Every request
  is unconditional.
- Resource files referenced by `ResourceRef.url` are downloaded by the
  generic `remote::fetch::fetch_to_cache` machinery, not by the Hub-specific
  client; that path is content-addressed and dedupe-friendly across
  `vex pull` and `vex hub install`.

## Future

Reserved for v2 of this protocol (not yet implemented):

- Pagination of `index.json` for large catalogs.
- Server-side search backend (`/search?q=...`) so clients don't have to
  download the full index.
- Signature / attestation envelope around `PublishedConfig` documents.

This protocol is implemented by `vex hub` subcommand (client).
