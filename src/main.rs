#![cfg_attr(all(target_os = "windows", not(debug_assertions)), windows_subsystem = "windows")]

mod ai;
mod runner;
mod theme;
mod workspace;

use eframe::egui;
use egui::{Color32, RichText, Stroke};
use serde_json::{json, Value};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
        Arc,
    },
    thread,
    time::Duration,
};

unsafe extern "C" {
    fn minux_engine_version() -> u32;
    fn minux_ease_out_cubic(progress: f32) -> f32;
    fn minux_ease_in_out_cubic(progress: f32) -> f32;
    fn minux_search_score(query: *const u8, query_len: usize, candidate: *const u8, candidate_len: usize) -> i32;
}

#[derive(Clone)]
struct ChatEntry {
    user: bool,
    content: String,
    reasoning: Option<String>,
}

enum AiEvent {
    AgentFinished(Result<ai::AgentResponse, String>),
    ModelsFinished(Result<Vec<String>, String>),
    RunFinished(Result<String, String>),
}

const BG: Color32 = Color32::from_rgb(18, 20, 26);
const PANEL: Color32 = Color32::from_rgb(23, 26, 33);
const PANEL_RAISED: Color32 = Color32::from_rgb(29, 33, 42);
const EDITOR_BG: Color32 = Color32::from_rgb(20, 23, 30);
const RAIL_BG: Color32 = Color32::from_rgb(16, 18, 24);
const BORDER: Color32 = Color32::from_rgb(43, 48, 60);
const TEXT: Color32 = Color32::from_rgb(222, 227, 236);
const MUTED: Color32 = Color32::from_rgb(137, 146, 163);
const ACCENT: Color32 = Color32::from_rgb(110, 155, 255);
const ACCENT_BG: Color32 = Color32::from_rgb(35, 48, 74);
const GREEN: Color32 = Color32::from_rgb(125, 201, 155);
const ORANGE: Color32 = Color32::from_rgb(230, 177, 112);
const RED: Color32 = Color32::from_rgb(229, 126, 137);

#[derive(PartialEq, Clone, Copy)]
enum SidebarView {
    Explorer,
    Search,
    Extensions,
}

struct MinuxIde {
    root: PathBuf,
    show_home: bool,
    selected_file: Option<PathBuf>,
    editor_text: String,
    dirty: bool,
    sidebar_view: SidebarView,
    settings_open: bool,
    show_ai: bool,
    show_output: bool,
    search_query: String,
    command_query: String,
    search_results: Vec<PathBuf>,
    chat_input: String,
    chat_messages: Vec<ChatEntry>,
    chat_history: Vec<Value>,
    ai_pending: bool,
    models_loading: bool,
    run_pending: bool,
    run_output: String,
    available_models: Vec<String>,
    theme_accent: String,
    custom_accent: String,
    corner_radius: u8,
    animations_enabled: bool,
    animation_speed: f32,
    editor_font_size: f32,
    recent_workspaces: Vec<String>,
    ai_tx: Sender<AiEvent>,
    ai_rx: Receiver<AiEvent>,
    status: String,
    api_key: String,
    model: String,
    thinking_enabled: bool,
    max_tokens: u32,
    icons: HashMap<&'static str, egui::TextureHandle>,
    icons_loaded: bool,
    tree_nodes: Vec<workspace::FileNode>,
    all_files: Vec<PathBuf>,
    visible_tree: Vec<workspace::TreeRow>,
    expanded_dirs: HashSet<PathBuf>,
    tree_rx: Option<Receiver<workspace::WorkspaceIndex>>,
    tree_cancel: Option<Arc<AtomicBool>>,
    tree_loading: bool,
    tree_truncated: bool,
}

impl Default for MinuxIde {
    fn default() -> Self {
        let settings = ai::load_settings();
        let root = settings.recent_workspaces.iter()
            .map(PathBuf::from)
            .find(|path| path.is_dir())
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_else(|| PathBuf::from("."));
        let (ai_tx, ai_rx) = mpsc::channel();

        Self {
            root,
            show_home: true,
            selected_file: None,
            editor_text: String::new(),
            dirty: false,
            sidebar_view: SidebarView::Explorer,
            settings_open: false,
            show_ai: false,
            show_output: false,
            search_query: String::new(),
            command_query: String::new(),
            search_results: Vec::new(),
            chat_input: String::new(),
            chat_messages: vec![ChatEntry {
                user: false,
                content: "MINUX Agent готов. Укажи Hugging Face token в настройках, выбери модель и отправь запрос. Агент может читать и изменять файлы открытого проекта.".into(),
                reasoning: None,
            }],
            chat_history: Vec::new(),
            ai_pending: false,
            models_loading: false,
            run_pending: false,
            run_output: String::new(),
            available_models: Vec::new(),
            theme_accent: if settings.theme_accent == "custom" || theme::valid_id(&settings.theme_accent) { settings.theme_accent.clone() } else { theme::default_id().to_owned() },
            custom_accent: settings.custom_accent,
            corner_radius: settings.corner_radius.clamp(3, 14),
            animations_enabled: settings.animations_enabled,
            animation_speed: settings.animation_speed.clamp(0.5, 2.0),
            editor_font_size: settings.editor_font_size.clamp(11.0, 20.0),
            recent_workspaces: settings.recent_workspaces,
            ai_tx,
            ai_rx,
            status: "Выбери проект, чтобы начать".into(),
            api_key: settings.token,
            model: settings.model,
            thinking_enabled: settings.thinking_enabled,
            max_tokens: settings.max_tokens,
            icons: HashMap::new(),
            icons_loaded: false,
            tree_nodes: Vec::new(),
            all_files: Vec::new(),
            visible_tree: Vec::new(),
            expanded_dirs: HashSet::new(),
            tree_rx: None,
            tree_cancel: None,
            tree_loading: false,
            tree_truncated: false,
        }
    }
}

fn apply_theme(ctx: &egui::Context, accent: Color32, accent_bg: Color32, radius: u8) {
    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(7.0, 6.0);
    style.spacing.button_padding = egui::vec2(10.0, 6.0);

    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = BG;
    visuals.window_fill = PANEL;
    visuals.extreme_bg_color = EDITOR_BG;
    visuals.faint_bg_color = PANEL_RAISED;
    visuals.code_bg_color = EDITOR_BG;
    visuals.hyperlink_color = accent;
    visuals.selection.bg_fill = accent_bg;
    visuals.selection.stroke = Stroke::new(1.0_f32, accent);
    visuals.widgets.noninteractive.corner_radius = egui::CornerRadius::same(radius);
    visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(radius);
    visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(radius);
    visuals.widgets.active.corner_radius = egui::CornerRadius::same(radius);
    visuals.widgets.open.corner_radius = egui::CornerRadius::same(radius);
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, BORDER);
    visuals.widgets.hovered.bg_fill = accent_bg;
    visuals.widgets.active.bg_fill = accent_bg;
    style.visuals = visuals;
    ctx.set_style(style);
}

fn load_icon_textures(ctx: &egui::Context) -> HashMap<&'static str, egui::TextureHandle> {
    let assets: [(&'static str, &'static str); 31] = [
        ("file-code", include_str!("../assets/icons/file-code.svg")),
        ("file-text", include_str!("../assets/icons/file-text.svg")),
        ("folder", include_str!("../assets/icons/folder.svg")),
        ("folder-open", include_str!("../assets/icons/folder-open.svg")),
        ("search", include_str!("../assets/icons/search.svg")),
        ("puzzle", include_str!("../assets/icons/puzzle.svg")),
        ("bot", include_str!("../assets/icons/bot.svg")),
        ("settings-2", include_str!("../assets/icons/settings-2.svg")),
        ("save", include_str!("../assets/icons/save.svg")),
        ("terminal", include_str!("../assets/icons/terminal.svg")),
        ("git-branch", include_str!("../assets/icons/git-branch.svg")),
        ("lang-rust", include_str!("../assets/icons/lang-rust.svg")),
        ("lang-typescript", include_str!("../assets/icons/lang-typescript.svg")),
        ("lang-javascript", include_str!("../assets/icons/lang-javascript.svg")),
        ("lang-python", include_str!("../assets/icons/lang-python.svg")),
        ("lang-c", include_str!("../assets/icons/lang-c.svg")),
        ("lang-cplusplus", include_str!("../assets/icons/lang-cplusplus.svg")),
        ("lang-csharp", include_str!("../assets/icons/lang-csharp.svg")),
        ("lang-bash", include_str!("../assets/icons/lang-bash.svg")),
        ("lang-xml", include_str!("../assets/icons/lang-xml.svg")),
        ("lang-html5", include_str!("../assets/icons/lang-html5.svg")),
        ("lang-json", include_str!("../assets/icons/lang-json.svg")),
        ("lang-make", include_str!("../assets/icons/lang-make.svg")),
        ("lang-css3", include_str!("../assets/icons/lang-css3.svg")),
        ("play", include_str!("../assets/icons/play.svg")),
        ("send", include_str!("../assets/icons/send.svg")),
        ("key-round", include_str!("../assets/icons/key-round.svg")),
        ("brain", include_str!("../assets/icons/brain.svg")),
        ("cloud-download", include_str!("../assets/icons/cloud-download.svg")),
        ("folder-plus", include_str!("../assets/icons/folder-plus.svg")),
        ("refresh-cw", include_str!("../assets/icons/refresh-cw.svg")),
    ];
    let mut icons = HashMap::with_capacity(assets.len() + 1);
    for (name, svg) in assets.into_iter().chain([("lang-yaml", include_str!("../assets/icons/lang-yaml.svg"))]) {
        let refined_svg = svg.replace("stroke-width=\"2\"", "stroke-width=\"1.45\"");
        match egui_extras::image::load_svg_bytes(refined_svg.as_bytes()) {
            Ok(image) => {
                icons.insert(name, ctx.load_texture(name, image, egui::TextureOptions::LINEAR));
            }
            Err(error) => eprintln!("MINUX IDE: icon {name} could not be loaded: {error}"),
        }
    }
    icons
}

fn paint_icon_at(
    ui: &egui::Ui,
    icons: &HashMap<&'static str, egui::TextureHandle>,
    name: &str,
    rect: egui::Rect,
    tint: Color32,
) {
    if let Some(texture) = icons.get(name) {
        ui.painter().image(
            texture.id(),
            rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            tint,
        );
    }
}

fn draw_icon(
    ui: &mut egui::Ui,
    icons: &HashMap<&'static str, egui::TextureHandle>,
    name: &str,
    size: f32,
    tint: Color32,
) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
    paint_icon_at(ui, icons, name, rect, tint);
}

fn toolbar_icon_button(
    ui: &mut egui::Ui,
    icons: &HashMap<&'static str, egui::TextureHandle>,
    name: &str,
    selected: bool,
    tooltip: &str,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(34.0, 32.0), egui::Sense::click());
    let accent = ui.style().visuals.hyperlink_color;
    let accent_bg = ui.style().visuals.selection.bg_fill;
    let hover_t = ui.ctx().animate_bool(response.id.with("hover"), response.hovered() || selected);
    if hover_t > 0.01 {
        let background = if selected { accent_bg } else { PANEL_RAISED };
        ui.painter().rect_filled(rect, egui::CornerRadius::same(5), background.gamma_multiply(hover_t));
    }
    let icon_rect = egui::Rect::from_center_size(rect.center(), egui::vec2(18.0, 18.0));
    paint_icon_at(ui, icons, name, icon_rect, if selected { accent } else { MUTED });
    response.on_hover_text(tooltip)
}

impl MinuxIde {
    fn open_workspace(&mut self) {
        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
            self.activate_workspace(folder);
        }
    }

    fn activate_workspace(&mut self, folder: PathBuf) {
        let folder = folder.canonicalize().unwrap_or(folder);
        if !folder.is_dir() {
            self.status = "Папка проекта не найдена.".into();
            return;
        }
        self.root = folder;
        self.show_home = false;
        self.settings_open = false;
        self.selected_file = None;
        self.editor_text.clear();
        self.dirty = false;
        let recent = self.root.to_string_lossy().into_owned();
        self.recent_workspaces.retain(|item| item != &recent);
        self.recent_workspaces.insert(0, recent);
        self.recent_workspaces.truncate(8);
        let _ = ai::save_settings(&self.current_ai_settings());
        self.status = "Сканирование проекта…".into();
        self.start_workspace_scan();
    }

    fn open_recent_workspace(&mut self, folder: &str) {
        self.activate_workspace(PathBuf::from(folder));
    }

    fn return_to_home(&mut self) {
        self.show_home = true;
        self.settings_open = false;
        self.show_ai = false;
        self.show_output = false;
        self.status = "Выбери проект, чтобы продолжить".into();
    }

    fn draw_home(&mut self, ui: &mut egui::Ui) {
        if self.settings_open {
            self.draw_settings(ui);
            return;
        }
        let recent = self.recent_workspaces.clone();
        let accent = self.accent_color();
        ui.vertical_centered(|ui| {
            ui.add_space((ui.available_height() * 0.10).max(28.0));
            ui.label(RichText::new("M").size(54.0).strong().color(accent));
            ui.add_space(2.0);
            ui.label(RichText::new("MINUX IDE").size(28.0).strong().color(TEXT));
            ui.add_space(7.0);
            ui.label(RichText::new("Рабочее пространство для твоего кода").size(14.0).color(MUTED));
            ui.add_space(26.0);
            let button_width = 240.0_f32.min((ui.available_width() - 30.0).max(150.0) / 2.0);
            ui.horizontal(|ui| {
                if ui.add_sized([button_width, 46.0], egui::Button::new(RichText::new("＋  Открыть проект").size(13.0))).clicked() {
                    self.open_workspace();
                }
                if ui.add_sized([button_width, 46.0], egui::Button::new(RichText::new("Настройки").size(13.0))).clicked() {
                    self.settings_open = true;
                }
            });
            ui.add_space(28.0);
            ui.set_min_width(520.0_f32.min(ui.available_width()));
            ui.label(RichText::new("ПОСЛЕДНИЕ ПРОЕКТЫ").size(10.0).strong().color(MUTED));
            ui.add_space(8.0);
            if recent.is_empty() {
                ui.label(RichText::new("Здесь появятся папки, которые ты открывал.").size(11.0).color(MUTED));
            } else {
                egui::ScrollArea::vertical()
                    .max_height((ui.available_height() - 20.0).max(100.0))
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for path in &recent {
                            let label = Path::new(path).file_name()
                                .map(|name| name.to_string_lossy().into_owned())
                                .unwrap_or_else(|| path.clone());
                            if ui.add_sized(
                                [ui.available_width().min(540.0), 36.0],
                                egui::Button::new(RichText::new(format!("{label}    ·    {path}")).size(11.5)).frame(true),
                            ).clicked() {
                                self.open_recent_workspace(path);
                            }
                        }
                    });
            }
            ui.add_space(12.0);
            ui.label(RichText::new("TypeScript · JavaScript · Python · Shell · C · Make · XSLT").size(11.0).color(MUTED));
            ui.add_space(10.0);
            ui.label(RichText::new(&self.status).size(10.5).color(MUTED));
        });
    }

    fn start_workspace_scan(&mut self) {
        if let Some(cancel) = self.tree_cancel.take() {
            cancel.store(true, Ordering::Relaxed);
        }
        self.tree_nodes.clear();
        self.all_files.clear();
        self.visible_tree.clear();
        self.expanded_dirs.clear();
        self.tree_truncated = false;
        self.tree_loading = true;

        let (tx, rx) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let cancel_for_worker = Arc::clone(&cancel);
        let root = self.root.clone();
        self.tree_rx = Some(rx);
        self.tree_cancel = Some(cancel);
        thread::spawn(move || {
            let index = workspace::scan_workspace(root, cancel_for_worker);
            let _ = tx.send(index);
        });
    }

    fn rebuild_visible_tree(&mut self) {
        self.visible_tree = workspace::flatten_visible(&self.tree_nodes, &self.expanded_dirs);
    }

    fn refresh_search_results(&mut self) {
        let needle = self.search_query.trim().to_lowercase();
        if needle.is_empty() {
            self.search_results.clear();
            return;
        }

        let mut ranked: Vec<(i32, PathBuf)> = self.all_files.iter().filter_map(|path| {
            let name = path.file_name().and_then(|name| name.to_str())?;
            let score = unsafe {
                minux_search_score(
                    needle.as_ptr(),
                    needle.len(),
                    name.as_bytes().as_ptr(),
                    name.len(),
                )
            };
            if score >= 0 {
                Some((score, path.clone()))
            } else if name.to_lowercase().contains(&needle) {
                Some((1, path.clone()))
            } else {
                None
            }
        }).collect();

        ranked.sort_by(|(score_a, path_a), (score_b, path_b)| {
            score_b.cmp(score_a).then_with(|| path_a.cmp(path_b))
        });
        self.search_results = ranked.into_iter().take(500).map(|(_, path)| path).collect();
    }

    fn open_file(&mut self, path: PathBuf) {
        if fs::metadata(&path).map(|meta| meta.len() > 4 * 1024 * 1024).unwrap_or(false) {
            self.status = "Файл больше 4 MiB. Открытие остановлено, чтобы интерфейс не зависал.".into();
            return;
        }
        match fs::read_to_string(&path) {
            Ok(text) => {
                self.editor_text = text;
                self.selected_file = Some(path);
                self.dirty = false;
                self.settings_open = false;
                self.status = "Файл открыт".into();
            }
            Err(e) => self.status = format!("Не удалось открыть файл: {e}"),
        }
    }

    fn current_ai_settings(&self) -> ai::AiSettings {
        ai::AiSettings {
            token: self.api_key.clone(),
            model: self.model.clone(),
            thinking_enabled: self.thinking_enabled,
            max_tokens: self.max_tokens,
            theme_accent: self.theme_accent.clone(),
            corner_radius: self.corner_radius,
            animations_enabled: self.animations_enabled,
            editor_font_size: self.editor_font_size,
            recent_workspaces: self.recent_workspaces.clone(),
            custom_accent: self.custom_accent.clone(),
            animation_speed: self.animation_speed,
        }
    }

    fn accent_color(&self) -> Color32 {
        if self.theme_accent == "custom" {
            theme::color(&self.custom_accent)
        } else {
            theme::accent(&self.theme_accent)
        }
    }

    fn accent_background(&self) -> Color32 {
        if self.theme_accent == "custom" {
            self.accent_color().gamma_multiply(0.24)
        } else {
            theme::accent_bg(&self.theme_accent)
        }
    }

    fn save_ai_settings(&mut self) {
        match ai::save_settings(&self.current_ai_settings()) {
            Ok(()) => self.status = "Настройки Hugging Face сохранены локально".into(),
            Err(error) => self.status = error,
        }
    }

    fn fetch_huggingface_models(&mut self) {
        if self.api_key.trim().is_empty() {
            self.status = "Сначала введи Hugging Face Access Token".into();
            self.settings_open = true;
            return;
        }
        let token = self.api_key.clone();
        let tx = self.ai_tx.clone();
        self.models_loading = true;
        self.status = "Загружаю доступные модели Hugging Face…".into();
        thread::spawn(move || {
            let result = ai::fetch_models(&token);
            let _ = tx.send(AiEvent::ModelsFinished(result));
        });
    }

    fn send_agent_prompt(&mut self, prompt: String) {
        if self.ai_pending || prompt.trim().is_empty() {
            return;
        }
        self.chat_messages.push(ChatEntry {
            user: true,
            content: prompt.clone(),
            reasoning: None,
        });

        if self.api_key.trim().is_empty() {
            self.chat_messages.push(ChatEntry {
                user: false,
                content: "Сначала открой Настройки, добавь Hugging Face Access Token и сохрани его.".into(),
                reasoning: None,
            });
            self.settings_open = true;
            self.status = "Не задан Hugging Face token".into();
            return;
        }
        if self.model.trim().is_empty() {
            self.chat_messages.push(ChatEntry {
                user: false,
                content: "Выбери модель Hugging Face или укажи её ID вручную.".into(),
                reasoning: None,
            });
            self.settings_open = true;
            return;
        }

        match ai::normalize_model_id(&self.model) {
            Ok(model) => self.model = model,
            Err(error) => {
                self.chat_messages.push(ChatEntry { user: false, content: error.clone(), reasoning: None });
                self.status = error;
                self.settings_open = true;
                return;
            }
        }
        self.chat_history.push(json!({"role":"user","content":prompt}));
        let settings = self.current_ai_settings();
        if let Err(error) = ai::save_settings(&settings) {
            self.status = format!("Запрос запущен, но настройки не сохранены: {error}");
        } else {
            self.status = format!("Запрос к {}…", settings.model);
        }
        let history = self.chat_history.clone();
        let root = self.root.clone();
        let tx = self.ai_tx.clone();
        self.ai_pending = true;
        thread::spawn(move || {
            let result = ai::run_agent(settings, history, root);
            let _ = tx.send(AiEvent::AgentFinished(result));
        });
    }

    fn run_selected_file(&mut self) {
        let Some(path) = self.selected_file.clone() else {
            self.status = "Сначала выбери файл для запуска.".into();
            return;
        };
        if self.run_pending {
            return;
        }
        if self.dirty {
            self.save_file();
            if self.dirty { return; }
        }
        self.run_pending = true;
        self.run_output = format!("Запуск {}…", path.display());
        self.show_output = true;
        self.status = "Выполняю выбранный файл в фоновом потоке…".into();
        let root = self.root.clone();
        let tx = self.ai_tx.clone();
        thread::spawn(move || {
            let result = runner::run_file(&root, &path);
            let _ = tx.send(AiEvent::RunFinished(result));
        });
    }

    fn save_file(&mut self) {
        let Some(path) = self.selected_file.clone() else {
            self.status = "Сначала выберите файл".into();
            return;
        };
        match fs::write(path, &self.editor_text) {
            Ok(()) => {
                self.dirty = false;
                self.status = "Файл сохранён".into();
            }
            Err(e) => self.status = format!("Ошибка сохранения: {e}"),
        }
    }

    fn draw_tree(&mut self, ui: &mut egui::Ui) {
        if self.tree_loading {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(RichText::new("Индексирую файлы в фоне…").size(11.0).color(MUTED));
            });
        }
        if self.tree_truncated {
            ui.label(RichText::new("Показана ограниченная часть большого проекта.").size(10.0).color(ORANGE));
        }
        let total_rows = self.visible_tree.len();
        let icons = &self.icons;
        let expanded_dirs = &self.expanded_dirs;
        let selected_file = self.selected_file.as_ref();
        let mut toggle_path: Option<PathBuf> = None;
        let mut selected_path: Option<PathBuf> = None;

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show_rows(ui, 23.0, total_rows, |ui, row_range| {
                let rows = self.visible_tree[row_range].to_vec();
                for row in rows {
                    ui.horizontal(|ui| {
                        ui.add_space((row.depth as f32 * 13.0) + 2.0);
                        if row.is_dir {
                            let expanded = expanded_dirs.contains(&row.path);
                            let arrow = if expanded { "▾" } else { "›" };
                            if ui.add_sized([13.0, 20.0], egui::Button::new(RichText::new(arrow).size(12.0).color(MUTED)).frame(false)).clicked() {
                                toggle_path = Some(row.path.clone());
                            }
                            draw_icon(ui, icons, if expanded { "folder-open" } else { "folder" }, 15.0, Color32::WHITE);
                        } else {
                            ui.add_space(13.0);
                            draw_icon(ui, icons, file_icon_key_for_path(&row.path), 15.0, Color32::WHITE);
                        }
                        let selected = selected_file == Some(&row.path);
                        let label = ui.add_sized(
                            [ui.available_width().max(30.0), 21.0],
                            egui::Label::new(
                                RichText::new(&row.name)
                                    .size(11.5)
                                    .color(if selected { TEXT } else { MUTED }),
                            ).sense(egui::Sense::click()),
                        );
                        if label.clicked() {
                            if row.is_dir {
                                toggle_path = Some(row.path.clone());
                            } else {
                                selected_path = Some(row.path.clone());
                            }
                        }
                    });
                }
            });

        if let Some(path) = toggle_path {
            if !self.expanded_dirs.remove(&path) {
                self.expanded_dirs.insert(path);
            }
            self.rebuild_visible_tree();
        }
        if let Some(path) = selected_path {
            self.open_file(path);
        }
    }

    fn draw_sidebar(&mut self, ui: &mut egui::Ui) {
        ui.add_space(7.0);
        match self.sidebar_view {
            SidebarView::Explorer => {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("ПРОВОДНИК").size(10.0).strong().color(MUTED));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if toolbar_icon_button(ui, &self.icons, "refresh-cw", false, "Пересканировать проект").clicked() {
                            self.start_workspace_scan();
                        }
                    });
                });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    draw_icon(ui, &self.icons, "folder-open", 16.0, Color32::WHITE);
                    ui.label(RichText::new(self.root.file_name().unwrap_or_default().to_string_lossy()).strong().size(12.0));
                });
                ui.separator();
                self.draw_tree(ui);
            }
            SidebarView::Search => {
                ui.vertical_centered(|ui| {
                    ui.horizontal(|ui| {
                        draw_icon(ui, &self.icons, "search", 15.0, MUTED);
                        ui.label(RichText::new("ПОИСК ФАЙЛОВ").size(10.0).strong().color(MUTED));
                    });
                });
                ui.add_space(8.0);
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.search_query)
                        .hint_text("Имя файла…")
                        .desired_width(f32::INFINITY),
                );
                if response.changed() {
                    self.refresh_search_results();
                }
                ui.add_space(8.0);
                if self.search_query.trim().is_empty() {
                    ui.label(RichText::new("Поиск по индексу проекта — без повторного обхода диска.").size(11.0).color(MUTED));
                } else {
                    let total = self.search_results.len();
                    ui.label(RichText::new(format!("РЕЗУЛЬТАТЫ · {total}{}", if total == 500 { "+" } else { "" })).size(10.0).strong().color(MUTED));
                    let icons = &self.icons;
                    let mut selected_path: Option<PathBuf> = None;
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show_rows(ui, 23.0, total, |ui, row_range| {
                            let rows = self.search_results[row_range].to_vec();
                            for path in rows {
                                let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                                ui.horizontal(|ui| {
                                    draw_icon(ui, icons, file_icon_key_for_path(&path), 15.0, Color32::WHITE);
                                    let response = ui.add_sized(
                                        [ui.available_width().max(30.0), 21.0],
                                        egui::Label::new(RichText::new(name).size(11.5).color(TEXT)).sense(egui::Sense::click()),
                                    );
                                    if response.clicked() {
                                        selected_path = Some(path.clone());
                                    }
                                });
                            }
                        });
                    if let Some(path) = selected_path {
                        self.open_file(path);
                    }
                }
            }
            SidebarView::Extensions => {
                ui.label(RichText::new("ЯЗЫКИ И ГРАММАТИКИ").size(10.0).strong().color(MUTED));
                ui.add_space(8.0);
                ui.label(RichText::new("Подсветка Syntect · встроенные грамматики").size(10.0).color(MUTED));
                ui.separator();
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .max_height((ui.available_height() - 100.0).max(170.0))
                    .show(ui, |ui| {
                        for (icon, label) in [
                            ("lang-rust", "Rust"),
                            ("lang-c", "C / C++"),
                            ("lang-csharp", "C#"),
                            ("lang-typescript", "TypeScript / TSX"),
                            ("lang-javascript", "JavaScript / JSX"),
                            ("lang-python", "Python"),
                            ("file-code", "Go"),
                            ("file-code", "Java"),
                            ("file-code", "Kotlin"),
                            ("file-code", "Swift"),
                            ("file-code", "Ruby"),
                            ("file-code", "PHP"),
                            ("file-code", "Lua"),
                            ("file-code", "Scala"),
                            ("file-code", "Haskell"),
                            ("file-code", "Clojure"),
                            ("file-code", "CoffeeScript"),
                            ("file-code", "F#"),
                            ("file-code", "Erlang"),
                            ("file-code", "OCaml"),
                            ("file-code", "MATLAB / R"),
                            ("lang-bash", "Shell / Bash"),
                            ("file-code", "SQL"),
                            ("lang-xml", "XML / XSL"),
                            ("lang-html5", "HTML"),
                            ("lang-css3", "CSS"),
                            ("lang-json", "JSON"),
                            ("lang-yaml", "YAML"),
                            ("file-code", "TOML / INI"),
                            ("file-text", "Markdown / LaTeX"),
                            ("file-code", "Diff / Patch"),
                            ("lang-make", "Makefile / CMake"),
                            ("file-code", "PowerShell"),
                            ("file-code", "Dockerfile"),
                        ] {
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                draw_icon(ui, &self.icons, icon, 16.0, Color32::WHITE);
                                ui.label(RichText::new(label).size(11.0).color(TEXT));
                            });
                        }
                    });
            }
        }
        ui.add_space(8.0);
        ui.separator();
        ui.horizontal(|ui| {
            ui.label(RichText::new("WORKSPACE").size(9.0).strong().color(MUTED));
            ui.label(RichText::new("LOCAL").size(9.0).color(GREEN));
        });
        ui.label(RichText::new(self.root.to_string_lossy()).size(10.0).color(MUTED));
    }

    fn draw_ai_sidebar(&mut self, ui: &mut egui::Ui) {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            draw_icon(ui, &self.icons, "bot", 19.0, self.accent_color());
            ui.label(RichText::new("MINUX Agent").strong().size(13.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("×").on_hover_text("Закрыть AI-панель").clicked() {
                    self.show_ai = false;
                }
            });
        });
        let connected = !self.api_key.trim().is_empty() && !self.model.trim().is_empty();
        ui.horizontal(|ui| {
            ui.label(RichText::new("●").color(if connected { GREEN } else { ORANGE }).size(10.0));
            ui.label(RichText::new(if connected { self.model.as_str() } else { "Добавь Hugging Face token в настройках" }).size(10.0).color(MUTED));
        });
        ui.separator();

        let scroll_height = (ui.available_height() - 135.0).max(100.0);
        egui::ScrollArea::vertical()
            .max_height(scroll_height)
            .stick_to_bottom(true)
            .show(ui, |ui| {
                for entry in &self.chat_messages {
                    ui.add_space(7.0);
                    ui.label(RichText::new(if entry.user { "ВЫ" } else { "MINUX AGENT" }).size(9.0).strong().color(if entry.user { ACCENT } else { GREEN }));
                    ui.add(egui::Label::new(RichText::new(&entry.content).size(12.0).color(TEXT)).wrap());
                    if let Some(reasoning) = &entry.reasoning {
                        ui.add_space(3.0);
                        egui::CollapsingHeader::new(RichText::new("Размышления модели").size(10.0).color(MUTED))
                            .default_open(false)
                            .show(ui, |ui| {
                                ui.add(egui::Label::new(RichText::new(reasoning).size(11.0).color(MUTED)).wrap());
                            });
                    }
                    ui.add_space(8.0);
                    ui.separator();
                }
                if self.ai_pending {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(RichText::new("Модель генерирует ответ…").size(11.0).color(MUTED));
                    });
                }
            });

        ui.add_space(5.0);
        if !connected && ui.button("Настроить Hugging Face").clicked() {
            self.settings_open = true;
        }
        ui.horizontal(|ui| {
            ui.label(RichText::new(if self.thinking_enabled { "Thinking: вкл." } else { "Thinking: выкл." }).size(10.0).color(MUTED));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if toolbar_icon_button(ui, &self.icons, "settings-2", false, "Настройки модели").clicked() {
                    self.settings_open = true;
                }
            });
        });

        let response = ui.add_sized(
            [ui.available_width(), 68.0],
            egui::TextEdit::multiline(&mut self.chat_input)
                .desired_rows(3)
                .hint_text("Опиши задачу, попроси изменить код или создать файлы…"),
        );
        ui.add_space(5.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Ctrl+Enter для отправки").size(9.0).color(MUTED));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let send_clicked = toolbar_icon_button(ui, &self.icons, "send", false, if self.ai_pending { "Запрос выполняется" } else { "Отправить сообщение" }).clicked();
                let send_enter = response.lost_focus()
                    && ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Enter));
                if (send_clicked || send_enter) && !self.chat_input.trim().is_empty() && !self.ai_pending {
                    let prompt = std::mem::take(&mut self.chat_input);
                    self.send_agent_prompt(prompt);
                }
            });
        });
    }

    fn draw_settings(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        ui.add_space(14.0);
        ui.label(RichText::new("Настройки").size(25.0).strong().color(TEXT));
        ui.label(RichText::new("Подключение модели и рабочая среда MINUX IDE.").size(12.0).color(MUTED));
        ui.add_space(18.0);
        ui.separator();
        ui.add_space(8.0);
        ui.label(RichText::new("HUGGING FACE INFERENCE").size(10.0).strong().color(self.accent_color()));
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            draw_icon(ui, &self.icons, "key-round", 16.0, self.accent_color());
            ui.label(RichText::new("Access Token").size(12.0).color(TEXT));
        });
        ui.add_sized(
            [ui.available_width().min(460.0), 34.0],
            egui::TextEdit::singleline(&mut self.api_key)
                .password(true)
                .hint_text("hf_…"),
        );
        ui.add_space(5.0);
        ui.label(RichText::new("Токен хранится локально в settings.json без шифрования. Не публикуй этот файл.").size(10.0).color(ORANGE));
        ui.add_space(16.0);
        ui.separator();
        ui.label(RichText::new("ОФОРМЛЕНИЕ").size(10.0).strong().color(self.accent_color()));
        ui.add_space(7.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Акцентный цвет").size(11.5).color(TEXT));
            egui::ComboBox::from_id_salt("minux_theme_accent")
                .selected_text(if self.theme_accent == "custom" { "Своя палитра" } else { theme::palette(&self.theme_accent).name.as_str() })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.theme_accent, "custom".to_owned(), "Своя палитра");
                    for palette in theme::all() {
                        ui.selectable_value(&mut self.theme_accent, palette.id.clone(), palette.name.as_str());
                    }
                });
        });
        ui.horizontal(|ui| {
            ui.label(RichText::new("Свой акцент").size(11.5).color(TEXT));
            let mut custom_color = theme::color(&self.custom_accent);
            if ui.color_edit_button_srgba(&mut custom_color).changed() {
                self.custom_accent = format!("#{:02X}{:02X}{:02X}", custom_color.r(), custom_color.g(), custom_color.b());
                self.theme_accent = "custom".into();
            }
            ui.label(RichText::new(&self.custom_accent).monospace().size(10.0).color(MUTED));
        });
        ui.add(egui::Slider::new(&mut self.corner_radius, 3..=14).text("Закругление"));
        ui.add(egui::Slider::new(&mut self.editor_font_size, 11.0..=20.0).step_by(0.5).text("Размер шрифта"));
        ui.add(egui::Slider::new(&mut self.animation_speed, 0.5..=2.0).step_by(0.1).text("Скорость переходов"));
        ui.checkbox(&mut self.animations_enabled, "Анимации панелей");
        ui.add_space(12.0);
        ui.separator();
        ui.add_space(8.0);

        ui.label(RichText::new("Модель").size(12.0).color(TEXT));
        let model_options: Vec<String> = if self.available_models.is_empty() {
            ai::suggested_models().iter().map(|item| (*item).to_owned()).collect()
        } else {
            self.available_models.clone()
        };
        ui.horizontal(|ui| {
            let model_width = (ui.available_width() - 148.0).max(130.0);
            ui.add_sized(
                [model_width, 34.0],
                egui::TextEdit::singleline(&mut self.model)
                    .hint_text("Организация/имя-модели"),
            );
            egui::ComboBox::from_id_salt("hf_model_selector")
                .selected_text("Модели ▾")
                .width(140.0)
                .show_ui(ui, |ui| {
                    for model in &model_options {
                        ui.selectable_value(&mut self.model, model.clone(), model);
                    }
                });
        });
        ui.add_space(5.0);
        ui.horizontal(|ui| {
            draw_icon(ui, &self.icons, "cloud-download", 16.0, self.accent_color());
            if ui.add_enabled(!self.models_loading, egui::Button::new(if self.models_loading { "Загрузка…" } else { "Загрузить доступные модели" })).clicked() {
                self.fetch_huggingface_models();
            }
            ui.label(RichText::new(format!("{} моделей", self.available_models.len())).size(10.0).color(MUTED));
        });
        ui.add_space(11.0);

        ui.horizontal(|ui| {
            draw_icon(ui, &self.icons, "brain", 16.0, self.accent_color());
            ui.label(RichText::new("Генерация").size(12.0).color(TEXT));
        });
        ui.checkbox(&mut self.thinking_enabled, "Thinking / extended reasoning (если поддерживается моделью)");
        ui.horizontal(|ui| {
            ui.label(RichText::new("Максимум токенов ответа").size(11.0).color(MUTED));
            egui::ComboBox::from_id_salt("hf_max_tokens")
                .selected_text(self.max_tokens.to_string())
                .show_ui(ui, |ui| {
                    for size in [512u32, 1024, 2048, 4096, 8192] {
                        ui.selectable_value(&mut self.max_tokens, size, size.to_string());
                    }
                });
        });
        ui.add_space(14.0);
        ui.horizontal(|ui| {
            if ui.button("Сохранить настройки").clicked() {
                self.save_ai_settings();
            }
            if ui.button("Проверить токен и модели").clicked() {
                self.fetch_huggingface_models();
            }
            if ui.button(if self.show_home { "Сначала выбери проект" } else { "Открыть AI-панель" }).clicked() {
                self.settings_open = false;
                if self.show_home {
                    self.status = "Сначала выбери проект на начальном экране.".into();
                } else {
                    self.show_ai = true;
                }
            }
        });
        ui.add_space(8.0);
        ui.label(RichText::new(&self.status).size(11.0).color(MUTED));
        ui.add_space(17.0);
        ui.separator();
        ui.add_space(10.0);
        ui.label(RichText::new("КОМПОНЕНТЫ").size(10.0).strong().color(ACCENT));
        settings_row(ui, "UI / Core", "Rust · egui", GREEN);
        settings_row(ui, "Syntax highlighting", "Syntect", GREEN);
        settings_row(ui, "SVG icons", "Lucide + Devicon", GREEN);
        settings_row(ui, "Native C core", &format!("C · v{}", unsafe { minux_engine_version() }), GREEN);
        settings_row(ui, "Runtime", "TypeScript · JS · Python · Shell · C · Make · XSLT", GREEN);
        ui.add_space(8.0);
        ui.label(RichText::new("Файловая структура сканируется в отдельном потоке; отображение дерева виртуализировано.").size(11.0).color(MUTED));
        ui.add_space(10.0);
        if ui.button(if self.show_home { "← На начальный экран" } else { "← Вернуться в редактор" }).clicked() {
            self.settings_open = false;
        }
        });
    }

    fn draw_welcome(&mut self, ui: &mut egui::Ui) {
        let spacer = (ui.available_height() * 0.14).max(18.0);
        ui.add_space(spacer);
        ui.vertical_centered(|ui| {
            ui.label(RichText::new("M").size(48.0).strong().color(ACCENT));
            ui.add_space(3.0);
            ui.label(RichText::new("MINUX IDE").size(25.0).strong().color(TEXT));
            ui.add_space(5.0);
            ui.label(RichText::new("Среда разработки для твоих проектов").size(13.0).color(MUTED));
            ui.add_space(28.0);
            if ui.button(RichText::new("＋   Открыть папку проекта").size(13.0)).clicked() {
                self.open_workspace();
            }
            ui.add_space(8.0);
            ui.label(RichText::new("Начните с открытия локальной папки").size(11.0).color(MUTED));
            ui.add_space(28.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Ctrl + O").monospace().size(10.0).color(ACCENT));
                ui.label(RichText::new("Открыть проект").size(11.0).color(MUTED));
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("Ctrl + S").monospace().size(10.0).color(ACCENT));
                ui.label(RichText::new("Сохранить файл").size(11.0).color(MUTED));
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("Ctrl + P").monospace().size(10.0).color(ACCENT));
                ui.label(RichText::new("Найти файл").size(11.0).color(MUTED));
            });
        });
    }

    fn draw_editor_workspace(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if let Some(path) = self.selected_file.clone() {
                let name = path.file_name().unwrap_or_default().to_string_lossy();
                draw_icon(ui, &self.icons, file_icon_key_for_path(&path), 17.0, Color32::WHITE);
                ui.label(RichText::new(name).size(12.0).color(TEXT));
                if self.dirty {
                    ui.label(RichText::new("●").color(ORANGE).size(9.0));
                }
                ui.separator();
                if ui.small_button("×").on_hover_text("Закрыть вкладку").clicked() {
                    self.selected_file = None;
                    self.editor_text.clear();
                    self.dirty = false;
                }
            } else {
                ui.label(RichText::new("Добро пожаловать").size(12.0).color(MUTED));
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("⋯").on_hover_text("Действия редактора").clicked() {
                    self.status = "Сочетания: Ctrl+O — открыть папку, Ctrl+S — сохранить, Ctrl+P — найти файл".into();
                }
            });
        });
        ui.separator();

        if let Some(path) = self.selected_file.clone() {
            ui.horizontal(|ui| {
                ui.label(RichText::new(self.root.file_name().unwrap_or_default().to_string_lossy()).size(10.0).color(MUTED));
                ui.label(RichText::new("›").color(MUTED));
                ui.label(RichText::new(path.strip_prefix(&self.root).unwrap_or(&path).to_string_lossy()).size(10.0).color(ACCENT));
            });
            ui.add_space(5.0);

            let code_theme = egui_extras::syntax_highlighting::CodeTheme::dark(self.editor_font_size);
            let editor_font_size = self.editor_font_size;
            let syntax = syntax_selector_for_path(&path).to_string();
            let mut layouter = move |ui: &egui::Ui, source: &str, wrap_width: f32| {
                let mut job = if source.len() > 500_000 {
                    egui::text::LayoutJob::simple(
                        source.to_owned(),
                        egui::FontId::monospace(editor_font_size),
                        TEXT,
                        wrap_width,
                    )
                } else {
                    egui_extras::syntax_highlighting::highlight(
                        ui.ctx(),
                        ui.style(),
                        &code_theme,
                        source,
                        &syntax,
                    )
                };
                job.wrap.max_width = wrap_width;
                ui.fonts(|fonts| fonts.layout_job(job))
            };

            let available = ui.available_size();
            let response = ui.add_sized(
                [available.x.max(80.0), available.y.max(100.0)],
                egui::TextEdit::multiline(&mut self.editor_text)
                    .font(egui::FontId::monospace(self.editor_font_size))
                    .desired_width(f32::INFINITY)
                    .desired_rows(30)
                    .code_editor()
                    .frame(false)
                    .layouter(&mut layouter),
            );
            if response.changed() {
                self.dirty = true;
                self.status = "Есть несохранённые изменения".into();
            }
        } else {
            self.draw_welcome(ui);
        }
    }

    fn draw_output_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("ПАНЕЛЬ ВЫВОДА").size(10.0).strong().color(MUTED));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("×").clicked() {
                    self.show_output = false;
                }
            });
        });
        ui.separator();
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("MINUX IDE  ›  {}", self.status)).size(11.0).color(MUTED));
            if self.run_pending { ui.spinner(); }
        });
        ui.add_space(5.0);
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.add(egui::Label::new(RichText::new(&self.run_output).monospace().size(11.5).color(TEXT)).wrap());
        });
    }
}

impl eframe::App for MinuxIde {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.icons_loaded {
            self.icons = load_icon_textures(ctx);
            self.icons_loaded = true;
        }

        let scan_result = self.tree_rx.as_ref().and_then(|rx| rx.try_recv().ok());
        if let Some(index) = scan_result {
            self.tree_rx = None;
            self.tree_cancel = None;
            self.tree_loading = false;
            self.tree_nodes = index.nodes;
            self.all_files = index.all_files;
            self.tree_truncated = index.truncated;
            self.rebuild_visible_tree();
            self.refresh_search_results();
            self.status = if self.tree_truncated {
                format!("Индекс готов: {} файлов; достигнут лимит сканирования", self.all_files.len())
            } else {
                format!("Индекс готов: {} файлов", self.all_files.len())
            };
        }

        let mut events = Vec::new();
        while let Ok(event) = self.ai_rx.try_recv() {
            events.push(event);
        }
        for event in events {
            match event {
                AiEvent::AgentFinished(result) => {
                    self.ai_pending = false;
                    match result {
                        Ok(answer) => {
                            if answer.workspace_changed {
                                self.start_workspace_scan();
                            }
                            self.chat_history = answer.history;
                            self.chat_messages.push(ChatEntry {
                                user: false,
                                content: answer.content,
                                reasoning: answer.reasoning,
                            });
                            self.status = "Ответ получен".into();
                        }
                        Err(error) => {
                            self.chat_history.push(json!({"role":"assistant","content":error.clone()}));
                            self.chat_messages.push(ChatEntry {
                                user: false,
                                content: error.clone(),
                                reasoning: None,
                            });
                            self.status = error;
                        }
                    }
                }
                AiEvent::ModelsFinished(result) => {
                    self.models_loading = false;
                    match result {
                        Ok(models) => {
                            let count = models.len();
                            self.available_models = models;
                            self.status = format!("Hugging Face: загружено моделей — {count}");
                        }
                        Err(error) => self.status = error,
                    }
                }
                AiEvent::RunFinished(result) => {
                    self.run_pending = false;
                    self.show_output = true;
                    match result {
                        Ok(output) => {
                            self.run_output = output;
                            self.status = "Выполнение завершено".into();
                        }
                        Err(error) => {
                            self.run_output = error.clone();
                            self.status = "Команда завершилась с ошибкой".into();
                        }
                    }
                }
            }
        }

        if self.tree_loading || self.ai_pending || self.models_loading || self.run_pending {
            ctx.request_repaint_after(Duration::from_millis(100));
        }

        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::S)) {
            self.save_file();
        }
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::O)) {
            self.open_workspace();
        }
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::P)) {
            self.sidebar_view = SidebarView::Search;
            self.settings_open = false;
        }

        apply_theme(ctx, self.accent_color(), self.accent_background(), self.corner_radius);

        let view_progress = if self.animations_enabled {
            let duration = 0.28 / self.animation_speed.clamp(0.5, 2.0);
            let raw = ctx.animate_bool_with_time(egui::Id::new("minux-view-transition"), !self.show_home, duration);
            unsafe { minux_ease_in_out_cubic(raw) }
        } else if self.show_home { 0.0 } else { 1.0 };

        if self.show_home {
            egui::CentralPanel::default()
                .frame(egui::Frame::new().fill(BG))
                .show(ctx, |ui| self.draw_home(ui));
            if self.animations_enabled && view_progress > 0.001 {
                paint_transition_overlay(ctx, (view_progress * 68.0) as u8);
                ctx.request_repaint_after(Duration::from_millis(16));
            }
            return;
        }

        egui::TopBottomPanel::top("main_toolbar")
            .exact_height(48.0)
            .frame(egui::Frame::new().fill(PANEL).stroke(Stroke::new(1.0_f32, BORDER)))
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.add_space(4.0);
                    ui.label(RichText::new("M").size(17.0).strong().color(ACCENT));
                    ui.label(RichText::new("MINUX").size(13.0).strong().color(TEXT));
                    ui.label(RichText::new("IDE").size(10.0).color(MUTED));
                    ui.separator();
                    ui.label(RichText::new(self.root.file_name().unwrap_or_default().to_string_lossy()).size(11.0).color(MUTED));
                    ui.add_space(6.0);
                    let reserved_width = 3.0 * 40.0 + 4.0 * 7.0 + 165.0;
                    let search_width = (ui.available_width() - reserved_width).clamp(150.0, 310.0);
                    let command_search = ui.add_sized(
                        [search_width, 29.0],
                        egui::TextEdit::singleline(&mut self.command_query)
                            .hint_text("Поиск файлов  Ctrl+P"),
                    );
                    if command_search.changed() {
                        self.search_query = self.command_query.clone();
                        self.refresh_search_results();
                        if !self.command_query.trim().is_empty() {
                            self.sidebar_view = SidebarView::Search;
                            self.settings_open = false;
                        }
                    }
                    if toolbar_icon_button(ui, &self.icons, "folder-open", false, "Открыть проект").clicked() {
                        self.open_workspace();
                    }
                    if toolbar_icon_button(ui, &self.icons, "play", false, if self.run_pending { "Выполняется…" } else { "Запустить выбранный файл" }).clicked() && !self.run_pending {
                        self.run_selected_file();
                    }
                    if toolbar_icon_button(ui, &self.icons, "save", false, "Сохранить файл").clicked() {
                        self.save_file();
                    }
                    if toolbar_icon_button(ui, &self.icons, "bot", self.show_ai, "AI Agent").clicked() {
                        self.show_ai = !self.show_ai;
                    }
                    if toolbar_icon_button(ui, &self.icons, "folder-plus", false, "Начальный экран / другой проект").clicked() {
                        self.return_to_home();
                    }
                });
            });

        egui::TopBottomPanel::bottom("status_bar")
            .exact_height(25.0)
            .frame(egui::Frame::new().fill(Color32::from_rgb(26, 39, 61)))
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.label(RichText::new("◆").size(10.0).color(ACCENT));
                    ui.label(RichText::new(self.status.clone()).size(10.0).color(TEXT));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if toolbar_icon_button(ui, &self.icons, "terminal", self.show_output, if self.show_output { "Скрыть вывод" } else { "Показать вывод" }).clicked() {
                            self.show_output = !self.show_output;
                        }
                        draw_icon(ui, &self.icons, "git-branch", 13.0, Color32::WHITE);
                        ui.label(RichText::new(format!("C v{}", unsafe { minux_engine_version() })).size(10.0).color(TEXT));
                        if let Some(path) = &self.selected_file {
                            ui.separator();
                            ui.label(RichText::new(path.extension().and_then(|s| s.to_str()).unwrap_or("text").to_uppercase()).size(10.0).color(TEXT));
                            ui.separator();
                            ui.label(RichText::new(language_display_name(&path)).size(10.0).color(TEXT));
                            ui.separator();
                            ui.label(RichText::new(format!("{} строк", self.editor_text.lines().count().max(1))).size(10.0).color(TEXT));
                        }
                    });
                });
            });

        egui::SidePanel::left("activity_rail")
            .exact_width(50.0)
            .resizable(false)
            .frame(egui::Frame::new().fill(RAIL_BG).stroke(Stroke::new(1.0_f32, BORDER)))
            .show(ctx, |ui| {
                ui.add_space(10.0);
                if activity_button(ui, &self.icons, "folder", self.sidebar_view == SidebarView::Explorer && !self.settings_open, "Проводник") .clicked() {
                    self.sidebar_view = SidebarView::Explorer;
                    self.settings_open = false;
                }
                if activity_button(ui, &self.icons, "search", self.sidebar_view == SidebarView::Search && !self.settings_open, "Поиск файлов") .clicked() {
                    self.sidebar_view = SidebarView::Search;
                    self.settings_open = false;
                }
                if activity_button(ui, &self.icons, "puzzle", self.sidebar_view == SidebarView::Extensions && !self.settings_open, "Языки и грамматики") .clicked() {
                    self.sidebar_view = SidebarView::Extensions;
                    self.settings_open = false;
                }
                ui.add_space(7.0);
                if activity_button(ui, &self.icons, "bot", self.show_ai, "AI Agent") .clicked() {
                    self.show_ai = !self.show_ai;
                }
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                    if activity_button(ui, &self.icons, "settings-2", self.settings_open, "Настройки") .clicked() {
                        self.settings_open = !self.settings_open;
                    }
                    ui.add_space(6.0);
                });
            });

        egui::SidePanel::left("workspace_sidebar")
            .default_width(238.0)
            .min_width(210.0)
            .max_width(320.0)
            .resizable(true)
            .frame(egui::Frame::new().fill(PANEL).stroke(Stroke::new(1.0_f32, BORDER)).inner_margin(egui::Margin::same(self.corner_radius as i8 + 4)))
            .show(ctx, |ui| self.draw_sidebar(ui));

        let ai_panel_progress = if self.animations_enabled {
            let duration = 0.30 / self.animation_speed.clamp(0.5, 2.0);
            let raw = ctx.animate_bool_with_time(egui::Id::new("minux-ai-sidebar-open"), self.show_ai, duration);
            if self.show_ai {
                unsafe { minux_ease_out_cubic(raw) }
            } else {
                unsafe { minux_ease_in_out_cubic(raw) }
            }
        } else if self.show_ai { 1.0 } else { 0.0 };
        if self.show_ai || ai_panel_progress > 0.01 {
            egui::SidePanel::right("ai_sidebar")
                .exact_width((320.0 * ai_panel_progress).max(1.0))
                .min_width(0.0)
                .max_width(410.0)
                .resizable(false)
                .frame(egui::Frame::new().fill(PANEL.gamma_multiply(ai_panel_progress.max(0.02))).stroke(Stroke::new(1.0_f32, BORDER)).inner_margin(egui::Margin::same(self.corner_radius as i8 + 4)))
                .show(ctx, |ui| self.draw_ai_sidebar(ui));
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(EDITOR_BG).inner_margin(egui::Margin::same(self.corner_radius as i8 + 6)))
            .show(ctx, |ui| {
                if self.settings_open {
                    self.draw_settings(ui);
                } else {
                    let available = ui.available_size();
                    if self.show_output {
                        let editor_height = (available.y - 118.0).max(180.0);
                        ui.allocate_ui_with_layout(
                            egui::vec2(available.x, editor_height),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| self.draw_editor_workspace(ui),
                        );
                        ui.separator();
                        self.draw_output_panel(ui);
                    } else {
                        self.draw_editor_workspace(ui);
                    }
                }
            });
        if self.animations_enabled && view_progress < 0.999 {
            paint_transition_overlay(ctx, ((1.0 - view_progress) * 68.0) as u8);
            ctx.request_repaint_after(Duration::from_millis(16));
        }
    }
}

fn paint_transition_overlay(ctx: &egui::Context, alpha: u8) {
    if alpha == 0 {
        return;
    }
    let screen = ctx.screen_rect();
    ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("minux-view-transition-overlay"),
    )).rect_filled(screen, egui::CornerRadius::same(0), Color32::from_black_alpha(alpha));
}

fn activity_button(
    ui: &mut egui::Ui,
    icons: &HashMap<&'static str, egui::TextureHandle>,
    icon_name: &str,
    selected: bool,
    tooltip: &str,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(40.0, 38.0), egui::Sense::click());
    let accent = ui.style().visuals.hyperlink_color;
    let accent_bg = ui.style().visuals.selection.bg_fill;
    let hover_t = ui.ctx().animate_bool(response.id.with("hover"), response.hovered() || selected);
    if hover_t > 0.01 {
        ui.painter().rect_filled(rect, egui::CornerRadius::same(5),
            (if selected { accent_bg } else { PANEL_RAISED }).gamma_multiply(hover_t));
    }
    if selected {
        let marker = egui::Rect::from_min_size(rect.left_top(), egui::vec2(2.0, rect.height()));
        ui.painter().rect_filled(marker, egui::CornerRadius::same(1), accent);
    }
    let icon_rect = egui::Rect::from_center_size(rect.center(), egui::vec2(20.0, 20.0));
    paint_icon_at(ui, icons, icon_name, icon_rect, if selected { accent } else { MUTED });
    response.on_hover_text(tooltip)
}

fn settings_row(ui: &mut egui::Ui, name: &str, value: &str, status_color: Color32) {
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new("●").size(9.0).color(status_color));
        ui.label(RichText::new(name).size(12.0).color(TEXT));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(value).size(11.0).color(MUTED));
        });
    });
}

fn file_icon_key_for_path(path: &Path) -> &'static str {
    let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("").to_ascii_lowercase();
    if matches!(name.as_str(), "makefile" | "gnumakefile" | "cmakelists.txt")
        || name.ends_with(".make")
        || name.ends_with(".mak")
    {
        return "lang-make";
    }
    if matches!(name.as_str(), ".bashrc" | ".bash_profile" | ".zshrc" | ".profile") {
        return "lang-bash";
    }

    match path.extension().and_then(|s| s.to_str()).unwrap_or("").to_ascii_lowercase().as_str() {
        "rs" => "lang-rust",
        "ts" | "tsx" => "lang-typescript",
        "js" | "jsx" | "mjs" | "cjs" => "lang-javascript",
        "py" | "pyw" => "lang-python",
        "c" => "lang-c",
        "h" => "lang-c",
        "cc" | "cpp" | "cxx" | "hpp" | "hxx" | "hh" => "lang-cplusplus",
        "cs" => "lang-csharp",
        "sh" | "bash" | "zsh" | "fish" => "lang-bash",
        "xml" | "xsl" | "xslt" | "xsd" | "dtd" => "lang-xml",
        "html" | "htm" => "lang-html5",
        "json" | "jsonc" => "lang-json",
        "md" | "markdown" | "txt" | "log" => "file-text",
        "css" | "scss" | "sass" => "lang-css3",
        "yml" | "yaml" => "lang-yaml",
        "mk" => "lang-make",
        _ => if path.is_dir() { "folder" } else { "file-code" },
    }
}

fn syntax_selector_for_path(path: &Path) -> &'static str {
    let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
    let lower_name = name.to_ascii_lowercase();
    if lower_name == "cmakelists.txt" {
        return "cmake";
    }
    if matches!(lower_name.as_str(), "makefile" | "gnumakefile")
        || lower_name.ends_with(".make")
        || lower_name.ends_with(".mak")
        || path.extension().and_then(|s| s.to_str()).is_some_and(|ext| ext.eq_ignore_ascii_case("mk"))
    {
        return "Makefile";
    }
    if matches!(lower_name.as_str(), ".bashrc" | ".bash_profile" | ".zshrc" | ".profile") {
        return "sh";
    }
    if lower_name == "dockerfile" {
        return "Dockerfile";
    }

    match path.extension().and_then(|s| s.to_str()).unwrap_or("").to_ascii_lowercase().as_str() {
        "rs" => "Rust",
        "ts" | "tsx" => "TypeScript",
        "js" | "jsx" | "mjs" | "cjs" => "JavaScript",
        "py" | "pyw" => "Python",
        "c" | "h" => "C",
        "cc" | "cpp" | "cxx" | "hpp" | "hxx" | "hh" => "C++",
        "cs" => "C#",
        "sh" | "bash" | "zsh" | "fish" => "sh",
        "xml" | "xsd" | "dtd" => "XML",
        "xsl" => "xsl",
        "xslt" => "xslt",
        "html" | "htm" => "HTML",
        "json" | "jsonc" => "JSON",
        "css" | "scss" | "sass" => "CSS",
        "yml" | "yaml" => "YAML",
        "toml" => "TOML",
        "md" | "markdown" => "Markdown",
        "sql" => "SQL",
        "java" => "Java",
        "go" => "Go",
        "lua" => "Lua",
        "php" => "PHP",
        "rb" => "Ruby",
        "swift" => "Swift",
        "kt" | "kts" => "Kotlin",
        "pl" | "pm" => "Perl",
        "ps1" | "psm1" => "PowerShell",
        "dockerfile" => "Dockerfile",
        "clj" | "cljs" | "cljc" | "edn" => "Clojure",
        "coffee" => "CoffeeScript",
        "d" => "D",
        "diff" | "patch" => "Diff",
        "erl" | "hrl" => "Erlang",
        "fs" | "fsx" => "F#",
        "groovy" | "gradle" => "Groovy",
        "hs" | "lhs" => "Haskell",
        "m" => "MATLAB",
        "mm" => "Objective-C",
        "ml" | "mli" => "OCaml",
        "r" => "R",
        "scala" | "sc" => "Scala",
        "tcl" => "Tcl",
        "tex" | "ltx" => "LaTeX",
        _ => "txt",
    }
}

fn language_display_name(path: &Path) -> &'static str {
    let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
    let lower_name = name.to_ascii_lowercase();
    if lower_name == "cmakelists.txt" {
        return "CMake";
    }
    if matches!(lower_name.as_str(), "makefile" | "gnumakefile")
        || lower_name.ends_with(".make")
        || lower_name.ends_with(".mak")
        || path.extension().and_then(|s| s.to_str()).is_some_and(|ext| ext.eq_ignore_ascii_case("mk"))
    {
        return "Makefile";
    }
    match path.extension().and_then(|s| s.to_str()).unwrap_or("").to_ascii_lowercase().as_str() {
        "rs" => "Rust",
        "ts" => "TypeScript",
        "tsx" => "TSX",
        "js" | "mjs" | "cjs" => "JavaScript",
        "jsx" => "JSX",
        "py" | "pyw" => "Python",
        "c" => "C",
        "h" => "C Header",
        "cc" | "cpp" | "cxx" => "C++",
        "hpp" | "hxx" | "hh" => "C++ Header",
        "cs" => "C#",
        "sh" | "bash" | "zsh" | "fish" => "Shell",
        "xml" => "XML",
        "xsl" | "xslt" => "XSLT",
        "xsd" => "XML Schema",
        "html" | "htm" => "HTML",
        "json" | "jsonc" => "JSON",
        "css" | "scss" | "sass" => "CSS",
        "yml" | "yaml" => "YAML",
        "toml" => "TOML",
        "md" | "markdown" => "Markdown",
        "sql" => "SQL",
        "java" => "Java",
        "go" => "Go",
        "lua" => "Lua",
        "php" => "PHP",
        "rb" => "Ruby",
        "swift" => "Swift",
        "kt" | "kts" => "Kotlin",
        "pl" | "pm" => "Perl",
        "ps1" | "psm1" => "PowerShell",
        "clj" | "cljs" | "cljc" | "edn" => "Clojure",
        "coffee" => "CoffeeScript",
        "d" => "D",
        "diff" | "patch" => "Diff",
        "erl" | "hrl" => "Erlang",
        "fs" | "fsx" => "F#",
        "groovy" | "gradle" => "Groovy",
        "hs" | "lhs" => "Haskell",
        "m" => "MATLAB",
        "mm" => "Objective-C",
        "ml" | "mli" => "OCaml",
        "r" => "R",
        "scala" | "sc" => "Scala",
        "tcl" => "Tcl",
        "tex" | "ltx" => "LaTeX",
        _ => "Text",
    }
}

#[cfg(test)]
mod native_search_engine_tests {
    use super::minux_search_score;

    fn score(query: &str, candidate: &str) -> i32 {
        unsafe {
            minux_search_score(query.as_ptr(), query.len(), candidate.as_ptr(), candidate.len())
        }
    }

    #[test]
    fn ranks_exact_and_prefix_matches_above_fuzzy_matches() {
        assert!(score("main.rs", "main.rs") > score("main", "main.rs"));
        assert!(score("main", "main.rs") > score("mnr", "main.rs"));
        assert_eq!(score("xyz", "main.rs"), -1);
    }
}

#[cfg(test)]
mod language_support_tests {
    use super::{language_display_name, syntax_selector_for_path};
    use std::path::Path;

    #[test]
    fn recognizes_additional_language_extensions() {
        let cases = [
            ("main.clj", "Clojure"),
            ("script.coffee", "CoffeeScript"),
            ("module.fs", "F#"),
            ("build.gradle", "Groovy"),
            ("Main.hs", "Haskell"),
            ("plot.m", "MATLAB"),
            ("bridge.mm", "Objective-C"),
            ("module.ml", "OCaml"),
            ("analysis.r", "R"),
            ("Main.scala", "Scala"),
            ("script.tcl", "Tcl"),
            ("paper.tex", "LaTeX"),
            ("change.patch", "Diff"),
        ];

        for (file, expected) in cases {
            let path = Path::new(file);
            assert_eq!(language_display_name(path), expected, "display name for {file}");
            assert_eq!(syntax_selector_for_path(path), expected, "syntax selector for {file}");
        }
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("MINUX IDE")
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([980.0, 620.0]),
        ..Default::default()
    };
    eframe::run_native(
        "MINUX IDE",
        options,
        Box::new(|_cc| {
            Ok(Box::new(MinuxIde::default()))
        }),
    )
}
