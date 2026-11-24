use maud::{html, Markup};
use oxarr_types::{EntityKind, Library, User};

use super::layout::base_layout;

pub fn libraries_page(
    libraries: &[Library],
    current_user: &User,
    notice: Option<&str>,
    error: Option<&str>,
) -> Markup {
    let content = html! {
        div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 2rem;" {
            div {
                h1 style="font-size: 2rem; margin-bottom: 0.5rem;" { "Media Libraries" }
                p style="color: var(--text-secondary);" { "Organized media paths monitored on disk" }
            }
            @if current_user.is_admin {
                button class="btn" onclick="document.getElementById('add-modal').style.display='flex'" {
                    "+ Add Library"
                }
            }
        }

        @if let Some(msg) = notice {
            div class="alert-success" { (msg) }
        }
        @if let Some(err) = error {
            div class="alert-error" { (err) }
        }

        div class="grid" {
            @for lib in libraries {
                div class="card" style="display: flex; flex-direction: column; justify-content: space-between;" {
                    div {
                        div style="display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: 0.75rem;" {
                            h3 style="font-size: 1.25rem;" { (lib.name) }
                            span class="badge" { (lib.kind.label()) }
                        }
                        p style="color: var(--text-secondary); font-size: 0.875rem; word-break: break-all; margin-bottom: 1rem;" {
                            "📂 " (lib.root_path)
                        }
                    }
                    div style="display: flex; flex-direction: column; gap: 0.75rem; margin-top: 1rem;" {
                        div style="display: flex; gap: 0.5rem;" {
                            a href=(format!("/libraries/{}", lib.id)) class="btn" style="text-decoration: none; text-align: center; flex: 1;" {
                                "View Media"
                            }
                            @if current_user.is_admin {
                                button class="btn" style="background: var(--bg-card); color: var(--text-primary); border: 1px solid var(--border-color);"
                                        hx-post=(format!("/libraries/{}/scan", lib.id))
                                        hx-swap="outerHTML" {
                                    "🔄 Scan"
                                }
                            }
                        }

                        @if current_user.is_admin {
                            div style="display: flex; gap: 0.5rem; align-items: flex-start; justify-content: space-between; border-top: 1px solid var(--border-color); padding-top: 0.75rem;" {
                                details style="flex: 1;" {
                                    summary class="btn" style="list-style: none; display: inline-block; background: var(--bg-card); color: var(--text-primary); border: 1px solid var(--border-color); padding: 0.35rem 0.75rem; font-size: 0.8rem;" {
                                        "Edit"
                                    }
                                    form method="POST" action=(format!("/libraries/{}/edit", lib.id))
                                         style="display: flex; flex-direction: column; gap: 0.6rem; margin-top: 0.75rem;" {
                                        div {
                                            label style="display: block; font-size: 0.75rem; margin-bottom: 0.25rem; color: var(--text-secondary);" {
                                                "Library Name"
                                            }
                                            input type="text" name="name" required value=(lib.name)
                                                   style="width: 100%; padding: 0.45rem; border-radius: 6px; border: 1px solid var(--border-color); background: var(--bg-primary); color: var(--text-primary); font-size: 0.85rem;";
                                        }
                                        div {
                                            label style="display: block; font-size: 0.75rem; margin-bottom: 0.25rem; color: var(--text-secondary);" {
                                                "Root Folder Path"
                                            }
                                            input type="text" name="root_path" required value=(lib.root_path)
                                                   style="width: 100%; padding: 0.45rem; border-radius: 6px; border: 1px solid var(--border-color); background: var(--bg-primary); color: var(--text-primary); font-size: 0.85rem;";
                                        }
                                        div {
                                            label style="display: block; font-size: 0.75rem; margin-bottom: 0.25rem; color: var(--text-secondary);" {
                                                "Media Type"
                                            }
                                            select name="kind" style="width: 100%; padding: 0.45rem; border-radius: 6px; border: 1px solid var(--border-color); background: var(--bg-primary); color: var(--text-primary); font-size: 0.85rem;" {
                                                @for kind in EntityKind::LIBRARY_KINDS {
                                                    option value=(kind.value()) selected[*kind == lib.kind] { (kind.label()) }
                                                }
                                            }
                                        }
                                        button type="submit" class="btn" style="padding: 0.4rem 0.75rem; font-size: 0.8rem;" {
                                            "Save Changes"
                                        }
                                    }
                                }
                                form method="POST"
                                     action=(format!("/libraries/{}/delete", lib.id))
                                     onsubmit="return confirm('Delete this library and its catalog records? Media files on disk will not be touched.');" {
                                    button type="submit" class="btn" style="background: var(--danger); padding: 0.35rem 0.75rem; font-size: 0.8rem;" {
                                        "Delete"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        @if libraries.is_empty() {
            div class="card" style="text-align: center; padding: 3rem; margin-top: 2rem;" {
                @if current_user.is_admin {
                    p style="color: var(--text-secondary); font-size: 1.1rem; margin-bottom: 1rem;" {
                        "No libraries configured yet."
                    }
                    button class="btn" onclick="document.getElementById('add-modal').style.display='flex'" {
                        "Create your first library"
                    }
                } @else {
                    p style="color: var(--text-secondary); font-size: 1.1rem;" {
                        "No libraries have been shared with your account yet."
                    }
                }
            }
        }

        @if current_user.is_admin {
            // Add Library Modal
            div id="add-modal" style="display: none; position: fixed; inset: 0; background: rgba(0,0,0,0.6); align-items: center; justify-content: center; z-index: 50;" {
                div class="card" style="width: 100%; max-width: 480px;" {
                    h2 style="margin-bottom: 1.5rem;" { "Add Media Library" }
                    form method="POST" action="/libraries" style="display: flex; flex-direction: column; gap: 1rem;" {
                        div {
                            label style="display: block; font-size: 0.875rem; margin-bottom: 0.4rem; color: var(--text-secondary);" {
                                "Library Name"
                            }
                            input type="text" name="name" required placeholder="e.g. 4K Movies"
                                   style="width: 100%; padding: 0.6rem; border-radius: 6px; border: 1px solid var(--border-color); background: var(--bg-primary); color: var(--text-primary);";
                        }
                        div {
                            label style="display: block; font-size: 0.875rem; margin-bottom: 0.4rem; color: var(--text-secondary);" {
                                "Root Folder Path"
                            }
                            input type="text" name="root_path" required placeholder="/media/movies"
                                   style="width: 100%; padding: 0.6rem; border-radius: 6px; border: 1px solid var(--border-color); background: var(--bg-primary); color: var(--text-primary);";
                        }
                        div {
                            label style="display: block; font-size: 0.875rem; margin-bottom: 0.4rem; color: var(--text-secondary);" {
                                "Media Type"
                            }
                            select name="kind" style="width: 100%; padding: 0.6rem; border-radius: 6px; border: 1px solid var(--border-color); background: var(--bg-primary); color: var(--text-primary);" {
                                @for kind in EntityKind::LIBRARY_KINDS {
                                    option value=(kind.value()) { (kind.label()) }
                                }
                            }
                        }
                        div style="display: flex; justify-content: flex-end; gap: 0.5rem; margin-top: 1rem;" {
                            button type="button" class="btn" style="background: transparent; color: var(--text-secondary); border: 1px solid var(--border-color);"
                                    onclick="document.getElementById('add-modal').style.display='none'" {
                                "Cancel"
                            }
                            button type="submit" class="btn" {
                                "Create Library"
                            }
                        }
                    }
                }
            }
        }
    };

    base_layout("Libraries", "libraries", current_user, content)
}
