# oxarr

> High-performance, unified media acquisition and streaming daemon written in Rust.

Oxarr consolidates the multi-service `*Arr` ecosystem (Sonarr, Radarr, Prowlarr) and streaming server capabilities (Jellyfin client compatibility) into a single, resource-efficient binary designed to idle in tens of megabytes on local NAS storage.

## Key Features

- **Resource Efficient:** Single binary running on an embedded SQLite database (WAL mode).
- **Hybrid Typed Catalog:** Shared `UUIDv7` entity identity paired with dedicated per-medium tables for movies, movie collections, TV/anime series, music, books, and custom Wasm media types.
- **Dual Plugin System:** Fast native providers for standard protocols (Torznab, TMDB) and sandboxed WebAssembly (Wasm via Extism) for community plugins.
- **Unified Release Engine:** Single parser and scoring pipeline for torrent/Usenet releases.
- **Modern Management UI:** Server-rendered hypermedia dashboard using Axum, Askama, and htmx.

## Development

Oxarr uses [devenv](https://devenv.sh) to provide a pinned Rust toolchain, foreign WASM targets, and SQLite tools.

```bash
# Enter development shell
devenv shell

# Check workspace crates
cargo check --workspace

# Run test suite
cargo test --workspace

# Run daemon
cargo run -p oxarr-server
```

## Documentation

- [Architecture & Design Principles](docs/architecture.md)
- [Roadmap & Milestones](https://github.com/bubuntux/oxarr/milestones)
