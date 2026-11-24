use maud::{html, Markup};
use oxarr_types::User;

use super::layout::{auth_layout, base_layout};

pub fn setup_page(error: Option<&str>) -> Markup {
    let content = html! {
        h1 style="font-size: 1.5rem; margin-bottom: 0.4rem; text-align: center;" { "Create Admin Account" }
        p style="color: var(--text-secondary); font-size: 0.875rem; text-align: center; margin-bottom: 1.5rem;" {
            "Welcome to oxarr. Create the initial administrator account to secure your server."
        }

        @if let Some(err) = error {
            div class="alert-error" { (err) }
        }

        form method="POST" action="/setup" style="display: flex; flex-direction: column; gap: 1rem;" {
            div {
                label style="display: block; font-size: 0.875rem; margin-bottom: 0.4rem; color: var(--text-secondary);" {
                    "Username"
                }
                input type="text" name="username" required autocomplete="username" autofocus placeholder="admin"
                       style="width: 100%; padding: 0.65rem; border-radius: 6px; border: 1px solid var(--border-color); background: var(--bg-primary); color: var(--text-primary);";
            }
            div {
                label style="display: block; font-size: 0.875rem; margin-bottom: 0.4rem; color: var(--text-secondary);" {
                    "Password (min 8 characters)"
                }
                input type="password" name="password" required minlength="8" autocomplete="new-password"
                       style="width: 100%; padding: 0.65rem; border-radius: 6px; border: 1px solid var(--border-color); background: var(--bg-primary); color: var(--text-primary);";
            }
            div {
                label style="display: block; font-size: 0.875rem; margin-bottom: 0.4rem; color: var(--text-secondary);" {
                    "Confirm Password"
                }
                input type="password" name="confirm_password" required minlength="8" autocomplete="new-password"
                       style="width: 100%; padding: 0.65rem; border-radius: 6px; border: 1px solid var(--border-color); background: var(--bg-primary); color: var(--text-primary);";
            }
            button type="submit" class="btn" style="width: 100%; margin-top: 0.5rem;" {
                "Create Admin Account"
            }
        }
    };

    auth_layout("Initial Setup", content)
}

pub fn login_page(error: Option<&str>, next: Option<&str>) -> Markup {
    let content = html! {
        h1 style="font-size: 1.5rem; margin-bottom: 0.4rem; text-align: center;" { "Sign In" }
        p style="color: var(--text-secondary); font-size: 0.875rem; text-align: center; margin-bottom: 1.5rem;" {
            "Sign in to access your media libraries"
        }

        @if let Some(err) = error {
            div class="alert-error" { (err) }
        }

        form method="POST" action="/login" style="display: flex; flex-direction: column; gap: 1rem;" {
            @if let Some(next_path) = next {
                input type="hidden" name="next" value=(next_path);
            }
            div {
                label style="display: block; font-size: 0.875rem; margin-bottom: 0.4rem; color: var(--text-secondary);" {
                    "Username"
                }
                input type="text" name="username" required autocomplete="username" autofocus
                       style="width: 100%; padding: 0.65rem; border-radius: 6px; border: 1px solid var(--border-color); background: var(--bg-primary); color: var(--text-primary);";
            }
            div {
                label style="display: block; font-size: 0.875rem; margin-bottom: 0.4rem; color: var(--text-secondary);" {
                    "Password"
                }
                input type="password" name="password" required autocomplete="current-password"
                       style="width: 100%; padding: 0.65rem; border-radius: 6px; border: 1px solid var(--border-color); background: var(--bg-primary); color: var(--text-primary);";
            }
            button type="submit" class="btn" style="width: 100%; margin-top: 0.5rem;" {
                "Sign In"
            }
        }
    };

    auth_layout("Sign In", content)
}

pub fn account_page(current_user: &User, notice: Option<&str>, error: Option<&str>) -> Markup {
    let content = html! {
        div style="margin-bottom: 2rem;" {
            h1 style="font-size: 2rem; margin-bottom: 0.5rem;" { "My Account" }
            p style="color: var(--text-secondary);" { "Manage your profile and password" }
        }

        @if let Some(msg) = notice {
            div class="alert-success" { (msg) }
        }
        @if let Some(err) = error {
            div class="alert-error" { (err) }
        }

        div style="display: flex; flex-direction: column; gap: 2rem; max-width: 560px;" {
            div class="card" {
                h2 style="font-size: 1.25rem; margin-bottom: 1rem;" { "Profile" }
                div style="display: grid; grid-template-columns: 140px 1fr; gap: 0.75rem; font-size: 0.95rem;" {
                    span style="color: var(--text-secondary);" { "Username" }
                    strong { (current_user.username) }
                    span style="color: var(--text-secondary);" { "Role" }
                    span {
                        @if current_user.is_admin {
                            span class="badge" style="color: #f59e0b; border-color: #f59e0b;" { "Administrator" }
                        } @else {
                            span class="badge" { "Standard User" }
                        }
                    }
                    span style="color: var(--text-secondary);" { "Created" }
                    span { (current_user.created_at.format("%Y-%m-%d").to_string()) }
                }
            }

            div class="card" {
                h2 style="font-size: 1.25rem; margin-bottom: 0.4rem;" { "Change Password" }
                p style="color: var(--text-secondary); font-size: 0.875rem; margin-bottom: 1.25rem;" {
                    "Updating your password revokes all existing sessions across devices."
                }
                form method="POST" action="/account/password" style="display: flex; flex-direction: column; gap: 1rem;" {
                    div {
                        label style="display: block; font-size: 0.875rem; margin-bottom: 0.4rem; color: var(--text-secondary);" {
                            "Current Password"
                        }
                        input type="password" name="current_password" required autocomplete="current-password"
                               style="width: 100%; padding: 0.6rem; border-radius: 6px; border: 1px solid var(--border-color); background: var(--bg-primary); color: var(--text-primary);";
                    }
                    div {
                        label style="display: block; font-size: 0.875rem; margin-bottom: 0.4rem; color: var(--text-secondary);" {
                            "New Password (min 8 characters)"
                        }
                        input type="password" name="new_password" required minlength="8" autocomplete="new-password"
                               style="width: 100%; padding: 0.6rem; border-radius: 6px; border: 1px solid var(--border-color); background: var(--bg-primary); color: var(--text-primary);";
                    }
                    div {
                        label style="display: block; font-size: 0.875rem; margin-bottom: 0.4rem; color: var(--text-secondary);" {
                            "Confirm New Password"
                        }
                        input type="password" name="confirm_password" required minlength="8" autocomplete="new-password"
                               style="width: 100%; padding: 0.6rem; border-radius: 6px; border: 1px solid var(--border-color); background: var(--bg-primary); color: var(--text-primary);";
                    }
                    div style="display: flex; justify-content: flex-end;" {
                        button type="submit" class="btn" { "Update Password" }
                    }
                }
            }
        }
    };

    base_layout("Account", "account", current_user, content)
}
