# oxarr

> High-performance, unified media acquisition and streaming daemon written in Rust.

Oxarr consolidates the multi-service `*Arr` ecosystem (Sonarr, Radarr, Prowlarr) and streaming server capabilities (Jellyfin client compatibility) into a single, resource-efficient binary designed to idle in tens of megabytes on local NAS storage.

## Key Features

- **Resource Efficient:** Single binary running on an embedded SQLite database (WAL mode).
- **Generalized Entity Model:** Universal hierarchy for movies, TV series, books, anime, and music.
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

# Run daemon
cargo run -p oxarr-server
```

## Documentation

- [Architecture & Design Principles](docs/architecture.md)
- [Roadmap & Milestones](https://github.com/bubuntux/oxarr/milestones)
