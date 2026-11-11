use maud::{html, Markup, PreEscaped, DOCTYPE};

const CSS_STYLES: &str = r#"
    :root[data-theme="dark"] {
        --bg-primary: #0f172a;
        --bg-secondary: #1e293b;
        --bg-card: #334155;
        --text-primary: #f8fafc;
        --text-secondary: #94a3b8;
        --border-color: #475569;
        --accent: #38bdf8;
        --accent-hover: #0ea5e9;
        --danger: #ef4444;
        --success: #22c55e;
    }

    :root[data-theme="light"] {
        --bg-primary: #f8fafc;
        --bg-secondary: #ffffff;
        --bg-card: #f1f5f9;
        --text-primary: #0f172a;
        --text-secondary: #64748b;
        --border-color: #cbd5e1;
        --accent: #0284c7;
        --accent-hover: #0369a1;
        --danger: #dc2626;
        --success: #16a34a;
    }

    * {
        box-sizing: border-box;
        margin: 0;
        padding: 0;
        font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
    }

    body {
        background-color: var(--bg-primary);
        color: var(--text-primary);
        display: flex;
        min-height: 100vh;
    }

    nav {
        width: 240px;
        background-color: var(--bg-secondary);
        border-right: 1px solid var(--border-color);
        padding: 1.5rem 1rem;
        display: flex;
        flex-direction: column;
        gap: 1rem;
    }

    .logo {
        font-size: 1.5rem;
        font-weight: 800;
        color: var(--accent);
        display: flex;
        align-items: center;
        gap: 0.5rem;
        margin-bottom: 1rem;
    }

    nav a {
        color: var(--text-secondary);
        text-decoration: none;
        padding: 0.75rem 1rem;
        border-radius: 8px;
        font-weight: 500;
        transition: all 0.15s ease;
    }

    nav a:hover, nav a.active {
        background-color: var(--bg-card);
        color: var(--text-primary);
    }

    .theme-toggle {
        margin-top: auto;
        padding: 0.5rem 1rem;
        border: 1px solid var(--border-color);
        background: transparent;
        color: var(--text-primary);
        border-radius: 8px;
        cursor: pointer;
        display: flex;
        align-items: center;
        justify-content: center;
        gap: 0.5rem;
    }

    main {
        flex: 1;
        padding: 2rem;
        overflow-y: auto;
    }

    .btn {
        background-color: var(--accent);
        color: white;
        border: none;
        padding: 0.6rem 1.2rem;
        border-radius: 6px;
        font-weight: 600;
        cursor: pointer;
        transition: background 0.15s;
    }

    .btn:hover {
        background-color: var(--accent-hover);
    }

    .card {
        background-color: var(--bg-secondary);
        border: 1px solid var(--border-color);
        border-radius: 12px;
        padding: 1.5rem;
    }

    .grid {
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
        gap: 1.5rem;
        margin-top: 1.5rem;
    }

    .badge {
        display: inline-block;
        padding: 0.2rem 0.5rem;
        border-radius: 4px;
        font-size: 0.75rem;
        font-weight: 700;
        background-color: var(--bg-card);
        color: var(--accent);
        border: 1px solid var(--border-color);
    }
"#;

const THEME_SCRIPT: &str = r#"
    function toggleTheme() {
        const html = document.documentElement;
        const current = html.getAttribute('data-theme');
        const next = current === 'dark' ? 'light' : 'dark';
        html.setAttribute('data-theme', next);
        localStorage.setItem('oxarr-theme', next);
    }

    const savedTheme = localStorage.getItem('oxarr-theme');
    if (savedTheme) {
        document.documentElement.setAttribute('data-theme', savedTheme);
    }
"#;

pub fn base_layout(title: &str, current_nav: &str, content: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" data-theme="dark" {
            head {
                meta charset="UTF-8";
                meta name="viewport" content="width=device-width, initial-scale=1.0";
                title { (title) " - oxarr" }
                script src="https://unpkg.com/htmx.org@2.0.4" {}
                style { (PreEscaped(CSS_STYLES)) }
            }
            body {
                nav {
                    div class="logo" { "🦀 oxarr" }
                    a href="/libraries" class=(if current_nav == "libraries" { "active" } else { "" }) {
                        "📁 Libraries"
                    }
                    a href="/settings" class=(if current_nav == "settings" { "active" } else { "" }) {
                        "⚙️ Settings"
                    }
                    button class="theme-toggle" onclick="toggleTheme()" {
                        "🌓 Toggle Theme"
                    }
                }
                main id="content" {
                    (content)
                }
                script {
                    (PreEscaped(THEME_SCRIPT))
                }
            }
        }
    }
}
