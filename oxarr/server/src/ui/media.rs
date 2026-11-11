use maud::{html, Markup};
use oxarr_types::{Entity, EntityDetails, Library, MediaAsset};

use super::layout::base_layout;

fn format_size(bytes: u64) -> String {
    const GIB: f64 = 1024.0 * 1024.0 * 1024.0;
    const MIB: f64 = 1024.0 * 1024.0;
    if bytes as f64 >= GIB {
        format!("{:.2} GB", (bytes as f64) / GIB)
    } else {
        format!("{:.1} MB", (bytes as f64) / MIB)
    }
}

pub fn media_cards(entities: &[Entity]) -> Markup {
    html! {
        @for item in entities {
            a href=(format!("/media/{}", item.id)) style="text-decoration: none; color: inherit; display: block;" {
                div class="card" style="display: flex; flex-direction: column; overflow: hidden; padding: 0; height: 100%; cursor: pointer;" {
                    div style="height: 220px; background: var(--bg-card); display: flex; align-items: center; justify-content: center; position: relative;" {
                        @if let Some(poster) = &item.metadata.poster_url {
                            img src=(poster) alt=(item.title) style="width: 100%; height: 100%; object-fit: cover;";
                        } @else {
                            span style="font-size: 3rem; color: var(--text-secondary);" { "🎬" }
                        }
                        span class="badge" style="position: absolute; top: 0.75rem; right: 0.75rem; background: rgba(0,0,0,0.7);" {
                            (item.kind.label())
                        }
                    }
                    div style="padding: 1rem; flex: 1; display: flex; flex-direction: column; justify-content: space-between;" {
                        div {
                            h3 style="font-size: 1.1rem; margin-bottom: 0.25rem;" { (item.title) }
                            p style="color: var(--text-secondary); font-size: 0.875rem; margin-bottom: 0.75rem;" {
                                @match &item.details {
                                    EntityDetails::Movie(m) => {
                                        (m.year.map(|y| y.to_string()).unwrap_or_default())
                                    }
                                    EntityDetails::Series(s) => {
                                        (s.year.map(|y| y.to_string()).unwrap_or_default())
                                    }
                                    _ => ""
                                }
                            }
                        }
                        div style="display: flex; gap: 0.35rem; flex-wrap: wrap;" {
                            @if item.monitored {
                                span class="badge" style="color: var(--success); border-color: var(--success);" {
                                    "Monitored"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn library_media_page(library: &Library, entities: &[Entity]) -> Markup {
    let content = html! {
        div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 2rem;" {
            div {
                a href="/libraries" style="color: var(--text-secondary); text-decoration: none; font-size: 0.875rem;" {
                    "← Back to Libraries"
                }
                h1 style="font-size: 2rem; margin-top: 0.25rem;" { (library.name) }
                p style="color: var(--text-secondary); font-size: 0.875rem;" { (library.root_path) }
            }
            button class="btn" hx-post=(format!("/libraries/{}/scan", library.id)) hx-swap="outerHTML" {
                "🔄 Scan Folder"
            }
        }

        // Instant Search Bar
        div style="margin-bottom: 1.5rem;" {
            input type="text" placeholder="Search titles, years, or codecs..."
                   hx-get=(format!("/libraries/{}/search", library.id))
                   hx-trigger="keyup changed delay:250ms"
                   hx-target="#media-list"
                   style="width: 100%; max-width: 400px; padding: 0.75rem 1rem; border-radius: 8px; border: 1px solid var(--border-color); background: var(--bg-secondary); color: var(--text-primary);";
        }

        div id="media-list" class="grid" {
            (media_cards(entities))
        }

        @if entities.is_empty() {
            div class="card" style="text-align: center; padding: 3rem; margin-top: 2rem;" {
                p style="color: var(--text-secondary); font-size: 1.1rem; margin-bottom: 1rem;" {
                    "No media detected in this folder yet."
                }
                button class="btn" hx-post=(format!("/libraries/{}/scan", library.id)) hx-swap="outerHTML" {
                    "Scan Now"
                }
            }
        }
    };

    base_layout(&library.name, "libraries", content)
}

pub fn media_detail_page(entity: &Entity, assets: &[MediaAsset]) -> Markup {
    let (year_display, runtime_display) = match &entity.details {
        EntityDetails::Movie(m) => (
            m.year.map(|y| y.to_string()).unwrap_or_default(),
            m.runtime_minutes.map(|r| format!("{r}m")).unwrap_or_default(),
        ),
        EntityDetails::Series(s) => (
            s.year.map(|y| y.to_string()).unwrap_or_default(),
            s.status.clone().unwrap_or_default(),
        ),
        EntityDetails::Collection(_) => ("".to_owned(), "".to_owned()),
    };

    let content = html! {
        div style="margin-bottom: 1.5rem;" {
            @if let Some(lib_id) = entity.library_id {
                a href=(format!("/libraries/{}", lib_id)) style="color: var(--text-secondary); text-decoration: none; font-size: 0.875rem;" {
                    "← Back to Library"
                }
            } @else {
                a href="/libraries" style="color: var(--text-secondary); text-decoration: none; font-size: 0.875rem;" {
                    "← Back to Libraries"
                }
            }
        }

        // Media Hero Header
        div class="card" style="position: relative; overflow: hidden; padding: 0; margin-bottom: 2rem;" {
            @if let Some(backdrop) = &entity.metadata.backdrop_url {
                div style=(format!("position: absolute; inset: 0; background-image: url('{}'); background-size: cover; background-position: center; opacity: 0.15; filter: blur(2px);", backdrop)) {}
            }

            div style="position: relative; z-index: 10; display: flex; flex-direction: row; gap: 2rem; padding: 2rem; flex-wrap: wrap;" {
                // Poster
                div style="width: 220px; flex-shrink: 0; border-radius: 8px; overflow: hidden; box-shadow: 0 8px 24px rgba(0,0,0,0.5); background: var(--bg-card); display: flex; align-items: center; justify-content: center;" {
                    @if let Some(poster) = &entity.metadata.poster_url {
                        img src=(poster) alt=(entity.title) style="width: 100%; height: auto; display: block; object-fit: cover;";
                    } @else {
                        span style="font-size: 4rem; color: var(--text-secondary); padding: 4rem 0;" { "🎬" }
                    }
                }

                // Metadata Details
                div style="flex: 1; min-width: 280px; display: flex; flex-direction: column; justify-content: space-between;" {
                    div {
                        div style="display: flex; gap: 0.5rem; align-items: center; margin-bottom: 0.5rem; flex-wrap: wrap;" {
                            span class="badge" style="background: var(--accent); color: white; border: none;" {
                                (entity.kind.label())
                            }
                            @if !year_display.is_empty() {
                                span style="color: var(--text-secondary); font-size: 1.1rem; font-weight: 500;" { (year_display) }
                            }
                            @if !runtime_display.is_empty() {
                                span style="color: var(--text-secondary); font-size: 0.95rem;" { "• " (runtime_display) }
                            }
                        }

                        h1 style="font-size: 2.5rem; margin-bottom: 0.25rem; line-height: 1.2;" { (entity.title) }
                        @if let Some(orig) = &entity.original_title {
                            p style="color: var(--text-secondary); font-style: italic; margin-bottom: 1rem;" {
                                "Original: " (orig)
                            }
                        }

                        // Genres
                        @if !entity.metadata.genres.is_empty() {
                            div style="display: flex; gap: 0.4rem; margin-bottom: 1.25rem; flex-wrap: wrap;" {
                                @for genre in &entity.metadata.genres {
                                    span class="badge" style="background: var(--bg-secondary); border-color: var(--border-color);" {
                                        (genre)
                                    }
                                }
                            }
                        }

                        // External IDs
                        div style="display: flex; gap: 0.5rem; margin-bottom: 1.25rem; flex-wrap: wrap;" {
                            @for (provider, ext_id) in &entity.external_ids {
                                @if provider == "imdb" {
                                    a href=(format!("https://www.imdb.com/title/{}", ext_id)) target="_blank" class="badge" style="text-decoration: none; background: #f5c518; color: #000; font-weight: bold; border: none;" {
                                        "IMDb: " (ext_id) " ↗"
                                    }
                                } @else if provider == "tmdb" {
                                    a href=(format!("https://www.themoviedb.org/movie/{}", ext_id)) target="_blank" class="badge" style="text-decoration: none; background: #01b4e4; color: #fff; font-weight: bold; border: none;" {
                                        "TMDB: " (ext_id) " ↗"
                                    }
                                } @else if provider == "tvmaze" {
                                    a href=(format!("https://www.tvmaze.com/shows/{}", ext_id)) target="_blank" class="badge" style="text-decoration: none; background: #3c948b; color: #fff; font-weight: bold; border: none;" {
                                        "TVMaze: " (ext_id) " ↗"
                                    }
                                } @else {
                                    span class="badge" { (provider) ": " (ext_id) }
                                }
                            }
                        }

                        // Overview / Synopsis
                        @if let Some(overview) = &entity.metadata.overview {
                            p style="color: var(--text-primary); font-size: 1rem; line-height: 1.6; max-width: 800px;" {
                                (overview)
                            }
                        } @else {
                            p style="color: var(--text-secondary); font-style: italic;" {
                                "No synopsis available."
                            }
                        }
                    }

                    div style="margin-top: 1.5rem; display: flex; gap: 0.5rem;" {
                        span class="badge" style="color: var(--success); border-color: var(--success);" {
                            @if entity.monitored { "● Monitored" } @else { "○ Unmonitored" }
                        }
                    }
                }
            }
        }

        // Files & Stream Inspection
        h2 style="font-size: 1.5rem; margin-bottom: 1rem;" {
            "Media Assets & Streams (" (assets.len()) ")"
        }

        @if assets.is_empty() {
            div class="card" style="text-align: center; padding: 2rem;" {
                p style="color: var(--text-secondary);" {
                    "No media files linked to this catalog record yet."
                }
            }
        }

        @for asset in assets {
            div class="card" style="margin-bottom: 1.5rem;" {
                div style="display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: 1rem; flex-wrap: wrap; gap: 0.5rem;" {
                    div {
                        span style="font-size: 1.1rem; font-weight: 600; word-break: break-all;" {
                            "📁 " (asset.path)
                        }
                        div style="display: flex; gap: 0.75rem; color: var(--text-secondary); font-size: 0.875rem; margin-top: 0.35rem; align-items: center; flex-wrap: wrap;" {
                            span { "Size: " (format_size(asset.size_bytes)) }
                            span { "• Imported: " (asset.imported_at.format("%Y-%m-%d %H:%M").to_string()) }
                            @if let Some(hash) = &asset.hash_xxh3 {
                                span class="badge" style="font-family: monospace; background: var(--bg-secondary);" {
                                    "XXH3: " (hash)
                                }
                            }
                        }
                    }
                }

                (render_streams_table(&asset.media_info))
            }
        }
    };

    base_layout(&entity.title, "libraries", content)
}

fn render_streams_table(media_info: &serde_json::Value) -> Markup {
    let streams = media_info.get("streams").and_then(|v| v.as_array());

    html! {
        @if let Some(streams) = streams {
            div style="overflow-x: auto;" {
                table style="width: 100%; border-collapse: collapse; font-size: 0.875rem; text-align: left;" {
                    thead {
                        tr style="border-bottom: 1px solid var(--border-color); color: var(--text-secondary);" {
                            th style="padding: 0.5rem 0.75rem;" { "Type" }
                            th style="padding: 0.5rem 0.75rem;" { "Codec" }
                            th style="padding: 0.5rem 0.75rem;" { "Stream Details & Specifications" }
                        }
                    }
                    tbody {
                        @for (idx, stream) in streams.iter().enumerate() {
                            (render_stream_row(idx, stream))
                        }
                    }
                }
            }
        } @else {
            p style="color: var(--text-secondary); font-size: 0.875rem; margin-top: 0.5rem;" {
                "Stream metadata not yet probed for this file."
            }
        }
    }
}

fn render_stream_row(idx: usize, stream: &serde_json::Value) -> Markup {
    let stream_type = stream.get("codec_type").and_then(|v| v.as_str()).unwrap_or("unknown");
    let codec = stream.get("codec_name").and_then(|v| v.as_str()).unwrap_or("unknown");
    let stream_index = stream.get("index").and_then(|v| v.as_u64()).map(|i| i as usize).unwrap_or(idx);

    let mut details = Vec::new();
    if stream_type == "video" {
        if let (Some(w), Some(h)) = (stream.get("width").and_then(|v| v.as_u64()), stream.get("height").and_then(|v| v.as_u64())) {
            details.push(format!("{w}x{h}"));
        }
        if let Some(r_fps) = stream.get("r_frame_rate").and_then(|v| v.as_str()) {
            details.push(format!("{r_fps} fps"));
        }
        if let Some(pix) = stream.get("pix_fmt").and_then(|v| v.as_str()) {
            details.push(pix.to_string());
        }
    } else if stream_type == "audio" {
        if let Some(channels) = stream.get("channels").and_then(|v| v.as_u64()) {
            let layout = match channels {
                8 => "7.1 surround".to_owned(),
                6 => "5.1 surround".to_owned(),
                2 => "Stereo".to_owned(),
                1 => "Mono".to_owned(),
                n => format!("{n} channels"),
            };
            details.push(layout);
        }
        if let Some(hz) = stream.get("sample_rate").and_then(|v| v.as_str()) {
            details.push(format!("{hz} Hz"));
        }
    }

    if let Some(lang) = stream.get("tags").and_then(|t| t.get("language")).and_then(|l| l.as_str()) {
        details.push(format!("[{lang}]"));
    }

    if let Some(disp) = stream.get("disposition") {
        if disp.get("forced").and_then(|v| v.as_i64()) == Some(1) {
            details.push("Forced".to_owned());
        }
        if disp.get("default").and_then(|v| v.as_i64()) == Some(1) {
            details.push("Default".to_owned());
        }
    }

    html! {
        tr style="border-bottom: 1px solid rgba(255,255,255,0.05);" {
            td style="padding: 0.6rem 0.75rem; white-space: nowrap;" {
                @if stream_type == "video" {
                    span class="badge" style="background: rgba(37, 99, 235, 0.2); color: #60a5fa; border: none;" {
                        "🎥 Video #" (stream_index)
                    }
                } @else if stream_type == "audio" {
                    span class="badge" style="background: rgba(16, 185, 129, 0.2); color: #34d399; border: none;" {
                        "🔊 Audio #" (stream_index)
                    }
                } @else if stream_type == "subtitle" {
                    span class="badge" style="background: rgba(245, 158, 11, 0.2); color: #fbbf24; border: none;" {
                        "💬 Subtitle #" (stream_index)
                    }
                } @else {
                    span class="badge" { "Stream #" (stream_index) }
                }
            }
            td style="padding: 0.6rem 0.75rem; font-weight: 600; text-transform: uppercase;" {
                (codec)
            }
            td style="padding: 0.6rem 0.75rem; color: var(--text-secondary);" {
                (details.join(" • "))
            }
        }
    }
}
