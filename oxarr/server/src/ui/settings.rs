use maud::{html, Markup};
use oxarr_types::{Library, User};
use uuid::Uuid;

use super::layout::base_layout;

pub fn settings_page(
    users: &[(User, Vec<Uuid>)],
    libraries: &[Library],
    current_user: &User,
    notice: Option<&str>,
    error: Option<&str>,
) -> Markup {
    let content = html! {
        div style="margin-bottom: 2rem;" {
            h1 style="font-size: 2rem; margin-bottom: 0.5rem;" { "System Settings" }
            p style="color: var(--text-secondary);" { "Manage users, library access permissions, and metadata providers" }
        }

        @if let Some(msg) = notice {
            div class="alert-success" { (msg) }
        }
        @if let Some(err) = error {
            div class="alert-error" { (err) }
        }

        div style="display: flex; flex-direction: column; gap: 2rem;" {
            // User Management Section
            div class="card" {
                div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 1.5rem;" {
                    div {
                        h2 style="font-size: 1.3rem;" { "User Accounts & Permissions" }
                        p style="color: var(--text-secondary); font-size: 0.875rem;" { "Manage who can access and view your media libraries" }
                    }
                    button class="btn" onclick="document.getElementById('add-user-modal').style.display='flex'" {
                        "+ Add User"
                    }
                }

                div style="overflow-x: auto;" {
                    table style="width: 100%; border-collapse: collapse; text-align: left;" {
                        thead {
                            tr style="border-bottom: 1px solid var(--border-color); color: var(--text-secondary); font-size: 0.875rem;" {
                                th style="padding: 0.75rem 0.5rem;" { "Username" }
                                th style="padding: 0.75rem 0.5rem;" { "Role" }
                                th style="padding: 0.75rem 0.5rem;" { "Accessible Libraries" }
                                th style="padding: 0.75rem 0.5rem;" { "Created" }
                                th style="padding: 0.75rem 0.5rem; text-align: right;" { "Actions" }
                            }
                        }
                        tbody {
                            @for (u, granted_ids) in users {
                                tr style="border-bottom: 1px solid var(--border-color); vertical-align: top;" {
                                    td style="padding: 1rem 0.5rem; font-weight: 600;" { (u.username) }
                                    td style="padding: 1rem 0.5rem;" {
                                        @if u.is_admin {
                                            span class="badge" style="color: #f59e0b; border-color: #f59e0b;" { "Administrator" }
                                        } @else {
                                            span class="badge" { "Standard User" }
                                        }
                                    }
                                    td style="padding: 1rem 0.5rem; color: var(--text-secondary); font-size: 0.875rem;" {
                                        @if u.is_admin {
                                            "All Libraries (Universal Access)"
                                        } @else if libraries.is_empty() {
                                            "No libraries configured"
                                        } @else {
                                            form method="POST" action=(format!("/settings/users/{}/libraries", u.id))
                                                 style="display: flex; flex-wrap: wrap; gap: 0.6rem; align-items: center;" {
                                                @for lib in libraries {
                                                    label style="display: inline-flex; align-items: center; gap: 0.3rem; color: var(--text-primary); cursor: pointer;" {
                                                        input type="checkbox"
                                                              name="library_ids"
                                                              value=(lib.id)
                                                              checked[granted_ids.contains(&lib.id)];
                                                        (lib.name)
                                                    }
                                                }
                                                button type="submit" class="btn" style="padding: 0.3rem 0.65rem; font-size: 0.75rem;" {
                                                    "Save Access"
                                                }
                                            }
                                        }
                                    }
                                    td style="padding: 1rem 0.5rem; color: var(--text-secondary); font-size: 0.875rem; white-space: nowrap;" {
                                        (u.created_at.format("%Y-%m-%d").to_string())
                                    }
                                    td style="padding: 1rem 0.5rem; text-align: right;" {
                                        div style="display: inline-flex; gap: 0.5rem; align-items: flex-start; flex-wrap: wrap; justify-content: flex-end;" {
                                            details {
                                                summary class="btn" style="list-style: none; background: var(--bg-card); color: var(--text-primary); border: 1px solid var(--border-color); padding: 0.35rem 0.7rem; font-size: 0.75rem;" {
                                                    "Reset Password"
                                                }
                                                form method="POST" action=(format!("/settings/users/{}/password", u.id))
                                                     style="display: flex; gap: 0.4rem; margin-top: 0.5rem; align-items: center;" {
                                                    input type="password" name="password" required minlength="8" placeholder="New password"
                                                           style="padding: 0.35rem 0.5rem; border-radius: 6px; border: 1px solid var(--border-color); background: var(--bg-primary); color: var(--text-primary); font-size: 0.8rem;";
                                                    button type="submit" class="btn" style="padding: 0.35rem 0.65rem; font-size: 0.75rem;" {
                                                        "Set"
                                                    }
                                                }
                                            }
                                            @if u.id != current_user.id {
                                                form method="POST" action=(format!("/settings/users/{}/delete", u.id)) {
                                                    button type="submit" class="btn" style="background: var(--danger); padding: 0.35rem 0.7rem; font-size: 0.75rem;" {
                                                        "Delete"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Metadata Providers Section
            div class="card" {
                h2 style="font-size: 1.3rem; margin-bottom: 0.5rem;" { "Metadata Providers" }
                p style="color: var(--text-secondary); font-size: 0.875rem; margin-bottom: 1rem;" {
                    "Aggregated providers for movie and TV show enrichment"
                }

                div style="display: flex; flex-direction: column; gap: 0.75rem;" {
                    div style="display: flex; justify-content: space-between; align-items: center; padding: 0.75rem; background: var(--bg-primary); border-radius: 8px;" {
                        div {
                            strong { "TMDB (The Movie Database)" }
                            div style="font-size: 0.75rem; color: var(--text-secondary);" {
                                "Movies & Shows: Posters, cast, overviews, ratings"
                            }
                        }
                        span class="badge" style="color: var(--success); border-color: var(--success);" { "Active" }
                    }
                    div style="display: flex; justify-content: space-between; align-items: center; padding: 0.75rem; background: var(--bg-primary); border-radius: 8px;" {
                        div {
                            strong { "TVMaze" }
                            div style="font-size: 0.75rem; color: var(--text-secondary);" {
                                "TV Shows: Zero-config fallback, episode guides"
                            }
                        }
                        span class="badge" style="color: var(--success); border-color: var(--success);" { "Active" }
                    }
                    div style="display: flex; justify-content: space-between; align-items: center; padding: 0.75rem; background: var(--bg-primary); border-radius: 8px;" {
                        div {
                            strong { "OMDb / IMDb" }
                            div style="font-size: 0.75rem; color: var(--text-secondary);" {
                                "IMDb ratings, Rotten Tomatoes, Metacritic scores"
                            }
                        }
                        span class="badge" { "Optional" }
                    }
                }
            }
        }

        // Add User Modal
        div id="add-user-modal" style="display: none; position: fixed; inset: 0; background: rgba(0,0,0,0.6); align-items: center; justify-content: center; z-index: 50;" {
            div class="card" style="width: 100%; max-width: 440px;" {
                h2 style="margin-bottom: 1.5rem;" { "Create User Account" }
                form method="POST" action="/settings/users" style="display: flex; flex-direction: column; gap: 1rem;" {
                    div {
                        label style="display: block; font-size: 0.875rem; margin-bottom: 0.4rem; color: var(--text-secondary);" { "Username" }
                        input type="text" name="username" required placeholder="alice"
                               style="width: 100%; padding: 0.6rem; border-radius: 6px; border: 1px solid var(--border-color); background: var(--bg-primary); color: var(--text-primary);";
                    }
                    div {
                        label style="display: block; font-size: 0.875rem; margin-bottom: 0.4rem; color: var(--text-secondary);" { "Password (min 8 characters)" }
                        input type="password" name="password" required minlength="8"
                               style="width: 100%; padding: 0.6rem; border-radius: 6px; border: 1px solid var(--border-color); background: var(--bg-primary); color: var(--text-primary);";
                    }
                    div style="display: flex; align-items: center; gap: 0.5rem;" {
                        input type="checkbox" id="is_admin" name="is_admin" value="true";
                        label for="is_admin" style="font-size: 0.875rem;" {
                            "Administrator (Access to all libraries & settings)"
                        }
                    }
                    div style="display: flex; justify-content: flex-end; gap: 0.5rem; margin-top: 1rem;" {
                        button type="button" class="btn" style="background: transparent; color: var(--text-secondary); border: 1px solid var(--border-color);"
                                onclick="document.getElementById('add-user-modal').style.display='none'" {
                            "Cancel"
                        }
                        button type="submit" class="btn" { "Create User" }
                    }
                }
            }
        }
    };

    base_layout("Settings", "settings", current_user, content)
}
