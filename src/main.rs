use gtk::prelude::*;
use tao::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy},
    platform::unix::WindowExtUnix,
    window::WindowBuilder,
};
use wry::{NewWindowResponse, PageLoadEvent, WebView, WebViewBuilder, WebViewBuilderExtUnix};

#[derive(Debug, Clone)]
enum BrowserEvent {
    UiReady,
    Navigate(String),
    Back,
    Forward,
    Reload,
    Home,
    NewTab(Option<String>),
    SwitchTab(usize),
    CloseTab(usize),
    ZoomDelta(f64),
    ZoomReset,
    ToggleDevtools,
    ThemeChanged { hue: u16, dark: bool },
    TabTitleChanged { id: usize, title: String },
    TabLoadChanged { id: usize, url: String, loading: bool },
}

struct Tab {
    id: usize,
    title: String,
    url: String,
    loading: bool,
    zoom: f64,
    container: gtk::Box,
    webview: WebView,
}

fn main() -> wry::Result<()> {
    let event_loop = EventLoopBuilder::<BrowserEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();

    let window = WindowBuilder::new()
        .with_title("Rust Chrome • Material You 🦀")
        .with_inner_size(LogicalSize::new(1360.0, 860.0))
        .with_min_inner_size(LogicalSize::new(640.0, 420.0))
        .build(&event_loop)
        .unwrap();

    // Разделяем окно на верхнюю панель (Material You Chrome) и область контента (Вкладки)
    let toolbar_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let content_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
    {
        let vbox = window.default_vbox().unwrap();
        toolbar_box.set_size_request(-1, 96);
        vbox.pack_start(&toolbar_box, false, false, 0);
        vbox.pack_start(&content_box, true, true, 0);
        vbox.show_all();
    }

    // Создаём WebView для верхней панели управления (Material You UI)
    let ui_proxy = proxy.clone();
    let ui_webview = WebViewBuilder::new()
        .with_html(UI_HTML)
        .with_ipc_handler(move |req| {
            if let Some(evt) = parse_ipc_message(req.body()) {
                let _ = ui_proxy.send_event(evt);
            }
        })
        .build_gtk(&toolbar_box)?;

    // Состояние браузера
    let mut tabs: Vec<Tab> = Vec::new();
    let mut next_tab_id: usize = 1;
    let mut active_id: usize = 1;
    let mut current_hue: u16 = 258; // Лавандовый Material You по умолчанию
    let mut is_dark: bool = true;

    // Открываем первую вкладку со стартовой страницей Material You
    let first_tab = create_tab(
        &content_box,
        &proxy,
        active_id,
        None,
        current_hue,
        is_dark,
    )?;
    tabs.push(first_tab);

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        let _keep_window_alive = &window;

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,

            Event::UserEvent(cmd) => match cmd {
                BrowserEvent::UiReady => {
                    sync_ui(&ui_webview, &tabs, active_id);
                }

                BrowserEvent::Navigate(input) => {
                    let trimmed = input.trim();
                    if trimmed.is_empty() {
                        return;
                    }
                    let target = resolve_input(trimmed);
                    if let Some(tab) = tabs.iter_mut().find(|t| t.id == active_id) {
                        if target == "material://home" {
                            tab.url = "material://home".to_string();
                            tab.title = "Новая вкладка".to_string();
                            tab.loading = false;
                            let _ = tab
                                .webview
                                .load_html(&render_new_tab_html(current_hue, is_dark));
                        } else {
                            tab.url = target.clone();
                            tab.loading = true;
                            let _ = tab.webview.load_url(&target);
                        }
                    }
                    sync_ui(&ui_webview, &tabs, active_id);
                }

                BrowserEvent::Back => {
                    if let Some(tab) = tabs.iter_mut().find(|t| t.id == active_id) {
                        if tab.webview.can_go_back().unwrap_or(false) {
                            let _ = tab.webview.go_back();
                        } else if tab.url != "material://home" {
                            tab.url = "material://home".to_string();
                            tab.title = "Новая вкладка".to_string();
                            tab.loading = false;
                            let _ = tab
                                .webview
                                .load_html(&render_new_tab_html(current_hue, is_dark));
                            sync_ui(&ui_webview, &tabs, active_id);
                        }
                    }
                }

                BrowserEvent::Forward => {
                    if let Some(tab) = tabs.iter().find(|t| t.id == active_id) {
                        let _ = tab.webview.go_forward();
                    }
                }

                BrowserEvent::Reload => {
                    if let Some(tab) = tabs.iter_mut().find(|t| t.id == active_id) {
                        if tab.url == "material://home" {
                            let _ = tab
                                .webview
                                .load_html(&render_new_tab_html(current_hue, is_dark));
                        } else {
                            tab.loading = true;
                            let _ = tab.webview.reload();
                            sync_ui(&ui_webview, &tabs, active_id);
                        }
                    }
                }

                BrowserEvent::Home => {
                    if let Some(tab) = tabs.iter_mut().find(|t| t.id == active_id) {
                        tab.url = "material://home".to_string();
                        tab.title = "Новая вкладка".to_string();
                        tab.loading = false;
                        let _ = tab
                            .webview
                            .load_html(&render_new_tab_html(current_hue, is_dark));
                    }
                    sync_ui(&ui_webview, &tabs, active_id);
                }

                BrowserEvent::NewTab(opt_url) => {
                    next_tab_id += 1;
                    let new_id = next_tab_id;
                    if let Ok(new_tab) = create_tab(
                        &content_box,
                        &proxy,
                        new_id,
                        opt_url,
                        current_hue,
                        is_dark,
                    ) {
                        tabs.push(new_tab);
                        active_id = new_id;
                        show_active_tab(&tabs, active_id);
                        sync_ui(&ui_webview, &tabs, active_id);
                    }
                }

                BrowserEvent::SwitchTab(id) => {
                    if tabs.iter().any(|t| t.id == id) {
                        active_id = id;
                        show_active_tab(&tabs, active_id);
                        sync_ui(&ui_webview, &tabs, active_id);
                    }
                }

                BrowserEvent::CloseTab(id) => {
                    if let Some(idx) = tabs.iter().position(|t| t.id == id) {
                        let removed = tabs.remove(idx);
                        removed.container.hide();
                        content_box.remove(&removed.container);

                        if tabs.is_empty() {
                            next_tab_id += 1;
                            let new_id = next_tab_id;
                            if let Ok(new_tab) = create_tab(
                                &content_box,
                                &proxy,
                                new_id,
                                None,
                                current_hue,
                                is_dark,
                            ) {
                                tabs.push(new_tab);
                                active_id = new_id;
                            }
                        } else if active_id == id {
                            let next_idx = idx.min(tabs.len() - 1);
                            active_id = tabs[next_idx].id;
                        }

                        show_active_tab(&tabs, active_id);
                        sync_ui(&ui_webview, &tabs, active_id);
                    }
                }

                BrowserEvent::ZoomDelta(delta) => {
                    if let Some(tab) = tabs.iter_mut().find(|t| t.id == active_id) {
                        tab.zoom = (tab.zoom + delta).clamp(0.5, 2.5);
                        let _ = tab.webview.zoom(tab.zoom);
                    }
                }

                BrowserEvent::ZoomReset => {
                    if let Some(tab) = tabs.iter_mut().find(|t| t.id == active_id) {
                        tab.zoom = 1.0;
                        let _ = tab.webview.zoom(1.0);
                    }
                }

                BrowserEvent::ToggleDevtools => {
                    #[cfg(debug_assertions)]
                    if let Some(tab) = tabs.iter().find(|t| t.id == active_id) {
                        if tab.webview.is_devtools_open() {
                            tab.webview.close_devtools();
                        } else {
                            tab.webview.open_devtools();
                        }
                    }
                }

                BrowserEvent::ThemeChanged { hue, dark } => {
                    current_hue = hue;
                    is_dark = dark;
                    for tab in &tabs {
                        apply_theme_to_tab(tab, current_hue, is_dark);
                    }
                }

                BrowserEvent::TabTitleChanged { id, title } => {
                    if let Some(tab) = tabs.iter_mut().find(|t| t.id == id) {
                        if tab.url == "material://home" {
                            tab.title = "Новая вкладка".to_string();
                        } else if !title.trim().is_empty() && title != "about:blank" {
                            tab.title = title;
                        }
                        sync_ui(&ui_webview, &tabs, active_id);
                    }
                }

                BrowserEvent::TabLoadChanged { id, url, loading } => {
                    if let Some(tab) = tabs.iter_mut().find(|t| t.id == id) {
                        tab.loading = loading;
                        if url == "about:blank" {
                            if tab.url != "material://home" {
                                tab.url = "material://home".to_string();
                                tab.title = "Новая вкладка".to_string();
                                let _ = tab
                                    .webview
                                    .load_html(&render_new_tab_html(current_hue, is_dark));
                            } else if !loading {
                                apply_theme_to_tab(tab, current_hue, is_dark);
                            }
                        } else if !url.is_empty() {
                            tab.url = url.clone();
                            if tab.title == "Новая вкладка" || tab.title.is_empty() {
                                tab.title = extract_domain(&url);
                            }
                        }
                        sync_ui(&ui_webview, &tabs, active_id);
                    }
                }
            },
            _ => (),
        }
    });
}

fn create_tab(
    content_box: &gtk::Box,
    proxy: &EventLoopProxy<BrowserEvent>,
    id: usize,
    initial_url: Option<String>,
    hue: u16,
    dark: bool,
) -> wry::Result<Tab> {
    let tab_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
    content_box.pack_start(&tab_box, true, true, 0);
    tab_box.show_all();

    let p_title = proxy.clone();
    let p_load = proxy.clone();
    let p_newwin = proxy.clone();
    let p_ipc = proxy.clone();

    let (url_state, title_state, loading_state, builder) = match initial_url {
        Some(u) if !u.is_empty() && u != "material://home" => (
            u.clone(),
            extract_domain(&u),
            true,
            WebViewBuilder::new().with_url(&u),
        ),
        _ => (
            "material://home".to_string(),
            "Новая вкладка".to_string(),
            false,
            WebViewBuilder::new().with_html(&render_new_tab_html(hue, dark)),
        ),
    };

    let webview = builder
        .with_ipc_handler(move |req| {
            if let Some(evt) = parse_ipc_message(req.body()) {
                let _ = p_ipc.send_event(evt);
            }
        })
        .with_document_title_changed_handler(move |title| {
            let _ = p_title.send_event(BrowserEvent::TabTitleChanged { id, title });
        })
        .with_on_page_load_handler(move |event, url| {
            let loading = matches!(event, PageLoadEvent::Started);
            let _ = p_load.send_event(BrowserEvent::TabLoadChanged { id, url, loading });
        })
        .with_new_window_req_handler(move |url, _features| {
            let _ = p_newwin.send_event(BrowserEvent::NewTab(Some(url)));
            NewWindowResponse::Deny
        })
        .build_gtk(&tab_box)?;

    Ok(Tab {
        id,
        title: title_state,
        url: url_state,
        loading: loading_state,
        zoom: 1.0,
        container: tab_box,
        webview,
    })
}

fn show_active_tab(tabs: &[Tab], active_id: usize) {
    for tab in tabs {
        if tab.id == active_id {
            tab.container.show_all();
            let _ = tab.webview.set_visible(true);
        } else {
            tab.container.hide();
            let _ = tab.webview.set_visible(false);
        }
    }
}

fn apply_theme_to_tab(tab: &Tab, hue: u16, dark: bool) {
    let script = format!(
        "if (window.applyMaterialTheme) {{ window.applyMaterialTheme({}, {}); }}",
        hue, dark
    );
    let _ = tab.webview.evaluate_script(&script);
}

fn render_new_tab_html(hue: u16, dark: bool) -> String {
    NEW_TAB_HTML
        .replace("__INIT_HUE__", &hue.to_string())
        .replace("__INIT_DARK__", if dark { "true" } else { "false" })
}

fn sync_ui(ui_webview: &WebView, tabs: &[Tab], active_id: usize) {
    let mut tabs_json = String::from("[");
    for (i, t) in tabs.iter().enumerate() {
        if i > 0 {
            tabs_json.push(',');
        }
        tabs_json.push_str(&format!(
            r#"{{"id":{},"title":"{}","url":"{}","loading":{}}}"#,
            t.id,
            js_escape(&t.title),
            js_escape(&t.url),
            t.loading
        ));
    }
    tabs_json.push(']');

    let script = format!(
        "if (window.syncState) {{ window.syncState({{ activeId: {}, tabs: {} }}); }}",
        active_id, tabs_json
    );
    let _ = ui_webview.evaluate_script(&script);
}

fn parse_ipc_message(msg: &str) -> Option<BrowserEvent> {
    match msg {
        "ui_ready" => Some(BrowserEvent::UiReady),
        "back" => Some(BrowserEvent::Back),
        "forward" => Some(BrowserEvent::Forward),
        "reload" => Some(BrowserEvent::Reload),
        "home" => Some(BrowserEvent::Home),
        "new_tab" => Some(BrowserEvent::NewTab(None)),
        "zoom_in" => Some(BrowserEvent::ZoomDelta(0.1)),
        "zoom_out" => Some(BrowserEvent::ZoomDelta(-0.1)),
        "zoom_reset" => Some(BrowserEvent::ZoomReset),
        "devtools" => Some(BrowserEvent::ToggleDevtools),
        _ => {
            let (cmd, rest) = msg.split_once(':')?;
            match cmd {
                "nav" => Some(BrowserEvent::Navigate(rest.to_string())),
                "switch_tab" => rest.parse::<usize>().ok().map(BrowserEvent::SwitchTab),
                "close_tab" => rest.parse::<usize>().ok().map(BrowserEvent::CloseTab),
                "theme" => {
                    let (h_str, d_str) = rest.split_once(':')?;
                    let hue = h_str.parse::<u16>().ok()?;
                    let dark = d_str == "1";
                    Some(BrowserEvent::ThemeChanged { hue, dark })
                }
                _ => None,
            }
        }
    }
}

fn resolve_input(input: &str) -> String {
    if input == "material://home" || input == "about:home" || input == "rust://home" {
        return "material://home".to_string();
    }
    if input.starts_with("http://") || input.starts_with("https://") {
        return input.to_string();
    }
    let looks_like_domain = !input.contains(' ')
        && (input.contains('.') || input.starts_with("localhost") || input.starts_with("127.0.0.1"));
    if looks_like_domain {
        format!("https://{}", input)
    } else {
        format!("https://www.google.com/search?q={}", url_encode(input))
    }
}

fn extract_domain(url: &str) -> String {
    url.split("://")
        .nth(1)
        .unwrap_or(url)
        .split('/')
        .next()
        .unwrap_or("Сайт")
        .to_string()
}

fn url_encode(input: &str) -> String {
    let mut out = String::new();
    for b in input.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

fn js_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '<' => out.push_str("\\u003c"),
            '>' => out.push_str("\\u003e"),
            _ => out.push(c),
        }
    }
    out
}

// ============================================================================
// ВЕРХНЯЯ ПАНЕЛЬ БРАУЗЕРА (MATERIAL YOU CHROME UI)
// ============================================================================
const UI_HTML: &str = r#"<!DOCTYPE html>
<html lang="ru">
<head>
<meta charset="UTF-8">
<style>
  :root {
    --hue: 258;
    --bg-strip: hsl(var(--hue), 30%, 9%);
    --bg-toolbar: hsl(var(--hue), 24%, 15%);
    --bg-omnibox: hsl(var(--hue), 28%, 10%);
    --bg-omnibox-hover: hsl(var(--hue), 26%, 12%);
    --bg-tab-hover: hsla(var(--hue), 30%, 75%, 0.08);
    --bg-btn-hover: hsla(var(--hue), 40%, 80%, 0.12);
    --primary: hsl(var(--hue), 85%, 78%);
    --primary-container: hsl(var(--hue), 45%, 26%);
    --on-primary-container: hsl(var(--hue), 90%, 90%);
    --text-main: hsl(var(--hue), 20%, 93%);
    --text-muted: hsl(var(--hue), 15%, 66%);
    --divider: hsla(var(--hue), 25%, 75%, 0.14);
  }

  body.light {
    --bg-strip: hsl(var(--hue), 42%, 88%);
    --bg-toolbar: hsl(var(--hue), 38%, 95%);
    --bg-omnibox: hsl(var(--hue), 35%, 89%);
    --bg-omnibox-hover: hsl(var(--hue), 35%, 86%);
    --bg-tab-hover: hsla(var(--hue), 40%, 30%, 0.08);
    --bg-btn-hover: hsla(var(--hue), 40%, 30%, 0.10);
    --primary: hsl(var(--hue), 68%, 42%);
    --primary-container: hsl(var(--hue), 75%, 84%);
    --on-primary-container: hsl(var(--hue), 80%, 18%);
    --text-main: hsl(var(--hue), 30%, 14%);
    --text-muted: hsl(var(--hue), 18%, 40%);
    --divider: hsla(var(--hue), 30%, 25%, 0.15);
  }

  * {
    box-sizing: border-box;
    margin: 0;
    padding: 0;
    user-select: none;
    -webkit-user-select: none;
    font-family: system-ui, -apple-system, "Segoe UI", Roboto, sans-serif;
  }

  body {
    background: var(--bg-strip);
    color: var(--text-main);
    height: 96px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    transition: background 0.25s ease, color 0.25s ease;
  }

  /* Полоса вкладок (Tab Strip) */
  .tab-strip {
    height: 42px;
    display: flex;
    align-items: flex-end;
    padding: 6px 10px 0 10px;
    gap: 4px;
    overflow-x: auto;
    scrollbar-width: none;
  }
  .tab-strip::-webkit-scrollbar { display: none; }

  .tab {
    height: 36px;
    min-width: 130px;
    max-width: 220px;
    flex: 1;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 10px 0 12px;
    border-radius: 14px 14px 0 0;
    font-size: 12.5px;
    font-weight: 500;
    color: var(--text-muted);
    cursor: pointer;
    position: relative;
    transition: background 0.18s ease, color 0.18s ease;
  }

  .tab:hover {
    background: var(--bg-tab-hover);
    color: var(--text-main);
  }

  .tab.active {
    background: var(--bg-toolbar);
    color: var(--text-main);
    font-weight: 600;
  }

  /* Плавные скругления у основания активной вкладки в стиле Chrome Material You */
  .tab.active::before,
  .tab.active::after {
    content: "";
    position: absolute;
    bottom: 0;
    width: 10px;
    height: 10px;
    pointer-events: none;
  }
  .tab.active::before {
    left: -10px;
    background: radial-gradient(circle at 0 0, transparent 10px, var(--bg-toolbar) 10.5px);
  }
  .tab.active::after {
    right: -10px;
    background: radial-gradient(circle at 100% 0, transparent 10px, var(--bg-toolbar) 10.5px);
  }

  .tab-icon {
    width: 15px;
    height: 15px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--primary);
  }

  .spinner {
    width: 13px;
    height: 13px;
    border: 2px solid var(--divider);
    border-top-color: var(--primary);
    border-radius: 50%;
    animation: spin 0.75s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .tab-title {
    flex: 1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .tab-close {
    width: 20px;
    height: 20px;
    border-radius: 50%;
    border: none;
    background: transparent;
    color: var(--text-muted);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    font-size: 14px;
    line-height: 1;
  }
  .tab-close:hover {
    background: var(--bg-btn-hover);
    color: var(--text-main);
  }

  .new-tab-btn {
    width: 30px;
    height: 30px;
    margin-bottom: 3px;
    margin-left: 2px;
    border-radius: 50%;
    border: none;
    background: transparent;
    color: var(--text-main);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    flex-shrink: 0;
    transition: background 0.15s;
  }
  .new-tab-btn:hover {
    background: var(--bg-btn-hover);
  }

  /* Нижняя панель навигации и Omnibox */
  .toolbar {
    height: 54px;
    background: var(--bg-toolbar);
    border-radius: 12px 12px 0 0;
    display: flex;
    align-items: center;
    padding: 0 10px;
    gap: 6px;
    position: relative;
    border-bottom: 1px solid var(--divider);
    transition: background 0.25s ease;
  }

  .nav-btn {
    width: 34px;
    height: 34px;
    border-radius: 50%;
    border: none;
    background: transparent;
    color: var(--text-main);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: background 0.15s ease, transform 0.1s ease;
    flex-shrink: 0;
  }
  .nav-btn:hover {
    background: var(--bg-btn-hover);
  }
  .nav-btn:active {
    transform: scale(0.92);
  }
  .nav-btn svg {
    width: 18px;
    height: 18px;
    fill: currentColor;
  }

  /* Пилюльный Omnibox (Material You) */
  .omnibox {
    flex: 1;
    height: 36px;
    background: var(--bg-omnibox);
    border-radius: 999px;
    display: flex;
    align-items: center;
    padding: 0 14px;
    gap: 10px;
    border: 1.5px solid transparent;
    transition: background 0.2s, border-color 0.2s, box-shadow 0.2s;
  }
  .omnibox:hover {
    background: var(--bg-omnibox-hover);
  }
  .omnibox:focus-within {
    border-color: var(--primary);
    background: var(--bg-omnibox);
    box-shadow: 0 0 0 3px hsla(var(--hue), 80%, 65%, 0.18);
  }

  .omni-badge {
    width: 22px;
    height: 22px;
    border-radius: 50%;
    background: var(--primary-container);
    color: var(--on-primary-container);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 12px;
    flex-shrink: 0;
  }

  .omnibox input {
    flex: 1;
    background: transparent;
    border: none;
    outline: none;
    color: var(--text-main);
    font-size: 13.5px;
    user-select: text;
    -webkit-user-select: text;
  }
  .omnibox input::placeholder {
    color: var(--text-muted);
  }

  /* Палитра Material You прямо в тулбаре */
  .palette-pill {
    display: flex;
    align-items: center;
    gap: 5px;
    background: var(--bg-omnibox);
    padding: 5px 9px;
    border-radius: 999px;
    margin-left: 2px;
  }

  .swatch {
    width: 16px;
    height: 16px;
    border-radius: 50%;
    border: 2px solid transparent;
    cursor: pointer;
    transition: transform 0.15s, border-color 0.15s;
  }
  .swatch:hover {
    transform: scale(1.18);
  }
  .swatch.active {
    border-color: var(--text-main);
    transform: scale(1.12);
  }

  /* Полоска прогресса загрузки */
  .loader-bar {
    position: absolute;
    bottom: 0;
    left: 0;
    height: 2.5px;
    width: 100%;
    background: linear-gradient(90deg, transparent, var(--primary), transparent);
    background-size: 200% 100%;
    animation: loadingMove 1.2s infinite linear;
    opacity: 0;
    pointer-events: none;
    transition: opacity 0.2s;
  }
  .loader-bar.visible {
    opacity: 1;
  }
  @keyframes loadingMove {
    0% { background-position: 200% 0; }
    100% { background-position: -200% 0; }
  }
</style>
</head>
<body>
  <div class="tab-strip" id="tabStrip"></div>

  <div class="toolbar">
    <button class="nav-btn" title="Назад" onclick="send('back')">
      <svg viewBox="0 0 24 24"><path d="M20 11H7.83l5.59-5.59L12 4l-8 8 8 8 1.41-1.41L7.83 13H20v-2z"/></svg>
    </button>
    <button class="nav-btn" title="Вперёд" onclick="send('forward')">
      <svg viewBox="0 0 24 24"><path d="M12 4l-1.41 1.41L16.17 11H4v2h12.17l-5.58 5.59L12 20l8-8z"/></svg>
    </button>
    <button class="nav-btn" title="Обновить" onclick="send('reload')">
      <svg viewBox="0 0 24 24"><path d="M17.65 6.35A7.958 7.958 0 0012 4c-4.42 0-7.99 3.58-7.99 8s3.57 8 7.99 8c3.73 0 6.84-2.55 7.73-6h-2.08A5.99 5.99 0 0112 18c-3.31 0-6-2.69-6-6s2.69-6 6-6c1.66 0 3.14.69 4.22 1.78L13 11h7V4l-2.35 2.35z"/></svg>
    </button>
    <button class="nav-btn" title="Домашняя страница Material You" onclick="send('home')">
      <svg viewBox="0 0 24 24"><path d="M10 20v-6h4v6h5v-8h3L12 3 2 12h3v8z"/></svg>
    </button>

    <div class="omnibox">
      <div class="omni-badge" id="securityBadge">✦</div>
      <input
        id="urlInput"
        type="text"
        placeholder="Введите запрос в Google или веб-адрес..."
        spellcheck="false"
        autocomplete="off"
      />
    </div>

    <button class="nav-btn" title="Уменьшить масштаб" onclick="send('zoom_out')">
      <svg viewBox="0 0 24 24"><path d="M19 13H5v-2h14v2z"/></svg>
    </button>
    <button class="nav-btn" title="Сбросить масштаб (100%)" onclick="send('zoom_reset')" style="font-size:11px;font-weight:700;">
      100%
    </button>
    <button class="nav-btn" title="Увеличить масштаб" onclick="send('zoom_in')">
      <svg viewBox="0 0 24 24"><path d="M19 13h-6v6h-2v-6H5v-2h6V5h2v6h6v2z"/></svg>
    </button>

    <!-- Живой выбор оттенка Material You -->
    <div class="palette-pill" title="Динамический цвет Material You">
      <div class="swatch active" data-hue="258" style="background:hsl(258,85%,70%)" onclick="setTheme(258, null)"></div>
      <div class="swatch" data-hue="210" style="background:hsl(210,85%,68%)" onclick="setTheme(210, null)"></div>
      <div class="swatch" data-hue="150" style="background:hsl(150,70%,55%)" onclick="setTheme(150, null)"></div>
      <div class="swatch" data-hue="28"  style="background:hsl(28,90%,65%)"  onclick="setTheme(28, null)"></div>
      <div class="swatch" data-hue="340" style="background:hsl(340,82%,68%)" onclick="setTheme(340, null)"></div>
    </div>

    <button class="nav-btn" id="themeModeBtn" title="Светлая / Тёмная тема" onclick="toggleDark()">
      🌙
    </button>

    <div class="loader-bar" id="loaderBar"></div>
  </div>

<script>
  let currentHue = 258;
  let isDark = true;

  function send(msg) {
    window.ipc.postMessage(msg);
  }

  const urlInput = document.getElementById('urlInput');
  urlInput.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') {
      const val = urlInput.value.trim();
      if (val) {
        send('nav:' + val);
        urlInput.blur();
      }
    }
  });

  urlInput.addEventListener('focus', () => {
    urlInput.select();
  });

  function setTheme(hue, darkOverride) {
    if (hue !== null) currentHue = hue;
    if (darkOverride !== null) isDark = darkOverride;

    document.documentElement.style.setProperty('--hue', currentHue);
    document.body.classList.toggle('light', !isDark);
    document.getElementById('themeModeBtn').textContent = isDark ? '🌙' : '☀️';

    document.querySelectorAll('.swatch').forEach(el => {
      el.classList.toggle('active', Number(el.dataset.hue) === currentHue);
    });

    send('theme:' + currentHue + ':' + (isDark ? '1' : '0'));
  }

  function toggleDark() {
    setTheme(null, !isDark);
  }

  window.syncState = function(state) {
    const strip = document.getElementById('tabStrip');
    strip.innerHTML = '';

    let activeTab = null;

    state.tabs.forEach(tab => {
      if (tab.id === state.activeId) activeTab = tab;

      const tabEl = document.createElement('div');
      tabEl.className = 'tab' + (tab.id === state.activeId ? ' active' : '');
      tabEl.onclick = () => send('switch_tab:' + tab.id);

      const iconEl = document.createElement('div');
      iconEl.className = 'tab-icon';
      if (tab.loading) {
        iconEl.innerHTML = '<div class="spinner"></div>';
      } else {
        iconEl.textContent = tab.url === 'material://home' ? '✦' : '🌐';
      }

      const titleEl = document.createElement('div');
      titleEl.className = 'tab-title';
      titleEl.textContent = tab.title || 'Новая вкладка';

      const closeBtn = document.createElement('button');
      closeBtn.className = 'tab-close';
      closeBtn.textContent = '×';
      closeBtn.title = 'Закрыть вкладку';
      closeBtn.onclick = (e) => {
        e.stopPropagation();
        send('close_tab:' + tab.id);
      };

      tabEl.appendChild(iconEl);
      tabEl.appendChild(titleEl);
      tabEl.appendChild(closeBtn);
      strip.appendChild(tabEl);
    });

    const addBtn = document.createElement('button');
    addBtn.className = 'new-tab-btn';
    addBtn.title = 'Новая вкладка';
    addBtn.innerHTML = '<svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor"><path d="M19 13h-6v6h-2v-6H5v-2h6V5h2v6h6v2z"/></svg>';
    addBtn.onclick = () => send('new_tab');
    strip.appendChild(addBtn);

    if (activeTab) {
      if (document.activeElement !== urlInput) {
        urlInput.value = activeTab.url === 'material://home' ? '' : activeTab.url;
      }
      const badge = document.getElementById('securityBadge');
      if (activeTab.url === 'material://home') {
        badge.textContent = '✦';
      } else if (activeTab.url.startsWith('https://')) {
        badge.textContent = '🔒';
      } else {
        badge.textContent = '🌐';
      }

      const loader = document.getElementById('loaderBar');
      loader.classList.toggle('visible', !!activeTab.loading);
    }
  };

  window.addEventListener('DOMContentLoaded', () => {
    send('ui_ready');
  });
</script>
</body>
</html>
"#;

// ============================================================================
// СТАРТОВАЯ СТРАНИЦА НОВОЙ ВКЛАДКИ (MATERIAL YOU START PAGE)
// ============================================================================
const NEW_TAB_HTML: &str = r#"<!DOCTYPE html>
<html lang="ru">
<head>
<meta charset="UTF-8">
<title>Новая вкладка</title>
<style>
  :root {
    --hue: __INIT_HUE__;
    --bg: hsl(var(--hue), 28%, 10%);
    --surface: hsl(var(--hue), 24%, 15%);
    --surface-hover: hsl(var(--hue), 28%, 20%);
    --primary: hsl(var(--hue), 85%, 76%);
    --primary-container: hsl(var(--hue), 45%, 24%);
    --on-primary-container: hsl(var(--hue), 95%, 92%);
    --text-main: hsl(var(--hue), 20%, 94%);
    --text-muted: hsl(var(--hue), 15%, 65%);
  }

  body.light {
    --bg: hsl(var(--hue), 38%, 94%);
    --surface: hsl(var(--hue), 35%, 88%);
    --surface-hover: hsl(var(--hue), 40%, 82%);
    --primary: hsl(var(--hue), 68%, 42%);
    --primary-container: hsl(var(--hue), 78%, 82%);
    --on-primary-container: hsl(var(--hue), 80%, 16%);
    --text-main: hsl(var(--hue), 30%, 14%);
    --text-muted: hsl(var(--hue), 18%, 42%);
  }

  * {
    box-sizing: border-box;
    margin: 0;
    padding: 0;
    font-family: system-ui, -apple-system, "Segoe UI", Roboto, sans-serif;
  }

  body {
    background: var(--bg);
    color: var(--text-main);
    min-height: 100vh;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 32px 20px;
    transition: background 0.3s ease, color 0.3s ease;
  }

  .hero-badge {
    background: var(--primary-container);
    color: var(--on-primary-container);
    padding: 6px 16px;
    border-radius: 999px;
    font-size: 13px;
    font-weight: 600;
    margin-bottom: 18px;
    letter-spacing: 0.3px;
  }

  .clock {
    font-size: 64px;
    font-weight: 800;
    letter-spacing: -2px;
    color: var(--primary);
    line-height: 1.05;
    margin-bottom: 8px;
  }

  .subtitle {
    color: var(--text-muted);
    font-size: 15px;
    margin-bottom: 32px;
  }

  .search-pill {
    width: 100%;
    max-width: 620px;
    height: 58px;
    background: var(--surface);
    border-radius: 999px;
    display: flex;
    align-items: center;
    padding: 0 22px;
    gap: 14px;
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.16);
    margin-bottom: 40px;
    border: 2px solid transparent;
    transition: border-color 0.2s, background 0.2s, transform 0.2s;
  }
  .search-pill:focus-within {
    border-color: var(--primary);
    transform: translateY(-2px);
  }

  .search-pill input {
    flex: 1;
    background: transparent;
    border: none;
    outline: none;
    color: var(--text-main);
    font-size: 16px;
  }
  .search-pill input::placeholder {
    color: var(--text-muted);
  }

  .search-btn {
    background: var(--primary-container);
    color: var(--on-primary-container);
    border: none;
    border-radius: 999px;
    padding: 9px 18px;
    font-weight: 600;
    font-size: 13.5px;
    cursor: pointer;
    transition: filter 0.15s;
  }
  .search-btn:hover {
    filter: brightness(1.1);
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(135px, 1fr));
    gap: 14px;
    width: 100%;
    max-width: 620px;
  }

  .card {
    background: var(--surface);
    border-radius: 24px;
    padding: 20px 14px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    cursor: pointer;
    transition: background 0.2s, transform 0.18s;
  }
  .card:hover {
    background: var(--surface-hover);
    transform: translateY(-4px);
  }

  .card-icon {
    width: 46px;
    height: 46px;
    border-radius: 16px;
    background: var(--primary-container);
    color: var(--on-primary-container);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 22px;
  }

  .card-title {
    font-size: 13.5px;
    font-weight: 600;
  }
</style>
</head>
<body>
  <div class="hero-badge">🦀 Rust Chrome • Material You (Arch Linux)</div>
  <div class="clock" id="clock">12:00</div>
  <div class="subtitle">Быстрый нативный браузер на Rust + WebKitGTK</div>

  <div class="search-pill">
    <span style="font-size:20px;color:var(--primary)">🔍</span>
    <input
      id="homeSearch"
      type="text"
      placeholder="Искать в Google или ввести адрес сайта..."
      autofocus
    />
    <button class="search-btn" onclick="submitSearch()">Найти</button>
  </div>

  <div class="grid">
    <div class="card" onclick="openUrl('https://www.google.com')">
      <div class="card-icon">G</div>
      <div class="card-title">Google</div>
    </div>
    <div class="card" onclick="openUrl('https://wiki.archlinux.org')">
      <div class="card-icon">🐧</div>
      <div class="card-title">ArchWiki</div>
    </div>
    <div class="card" onclick="openUrl('https://www.youtube.com')">
      <div class="card-icon">▶️</div>
      <div class="card-title">YouTube</div>
    </div>
    <div class="card" onclick="openUrl('https://github.com')">
      <div class="card-icon">🐙</div>
      <div class="card-title">GitHub</div>
    </div>
    <div class="card" onclick="openUrl('https://crates.io')">
      <div class="card-icon">📦</div>
      <div class="card-title">Crates.io</div>
    </div>
    <div class="card" onclick="openUrl('https://doc.rust-lang.org')">
      <div class="card-icon">🦀</div>
      <div class="card-title">Rust Docs</div>
    </div>
    <div class="card" onclick="openUrl('https://aur.archlinux.org')">
      <div class="card-icon">⚡</div>
      <div class="card-title">AUR</div>
    </div>
    <div class="card" onclick="openUrl('https://news.ycombinator.com')">
      <div class="card-icon">Y</div>
      <div class="card-title">Hacker News</div>
    </div>
  </div>

<script>
  window.applyMaterialTheme = function(hue, dark) {
    document.documentElement.style.setProperty('--hue', hue);
    document.body.classList.toggle('light', !dark);
  };

  window.applyMaterialTheme(__INIT_HUE__, __INIT_DARK__);

  function updateClock() {
    const now = new Date();
    const hh = String(now.getHours()).padStart(2, '0');
    const mm = String(now.getMinutes()).padStart(2, '0');
    document.getElementById('clock').textContent = hh + ':' + mm;
  }
  updateClock();
  setInterval(updateClock, 10000);

  function openUrl(u) {
    window.ipc.postMessage('nav:' + u);
  }

  function submitSearch() {
    const val = document.getElementById('homeSearch').value.trim();
    if (val) openUrl(val);
  }

  document.getElementById('homeSearch').addEventListener('keydown', (e) => {
    if (e.key === 'Enter') submitSearch();
  });
</script>
</body>
</html>
"#;
