use eframe::egui;
use egui::{Color32, RichText, Stroke};

unsafe extern "C" {
    fn minux_engine_version() -> u32;
}

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

#[cfg(target_os = "windows")]
const CSHARP_AGENT_BINARY: &[u8] = include_bytes!(env!("MINUX_AGENT_EXE_PATH"));

#[cfg(target_os = "windows")]
fn agent_version() -> u32 {
    1
}

#[cfg(not(target_os = "windows"))]
fn agent_version() -> u32 {
    0
}

#[cfg(target_os = "windows")]
fn run_csharp_agent(prompt: &str) -> Result<String, String> {
    use std::{
        io::Write,
        os::windows::process::CommandExt,
        process::{Command, Stdio},
    };

    let agent_path = std::env::temp_dir().join(format!(
        "MINUXAgent-{}.exe",
        env!("MINUX_AGENT_FINGERPRINT")
    ));

    if !agent_path.is_file() {
        fs::write(&agent_path, CSHARP_AGENT_BINARY)
            .map_err(|e| format!("Не удалось извлечь C# Agent: {e}"))?;
    }

    let mut child = Command::new(&agent_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(0x08000000)
        .spawn()
        .map_err(|e| format!("Не удалось запустить C# Agent: {e}"))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(prompt.as_bytes())
            .map_err(|e| format!("Не удалось передать запрос C# Agent: {e}"))?;
    }

    let output = child.wait_with_output()
        .map_err(|e| format!("Не удалось получить ответ C# Agent: {e}"))?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if error.is_empty() {
            format!("C# Agent завершился с кодом {}", output.status)
        } else {
            error
        });
    }

    let answer = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if answer.is_empty() {
        Err("C# Agent вернул пустой ответ".into())
    } else {
        Ok(answer)
    }
}

#[cfg(not(target_os = "windows"))]
fn run_csharp_agent(_prompt: &str) -> Result<String, String> {
    Err("C# NativeAOT Agent включён только в сборку Windows.".into())
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
    selected_file: Option<PathBuf>,
    editor_text: String,
    dirty: bool,
    sidebar_view: SidebarView,
    settings_open: bool,
    show_ai: bool,
    show_output: bool,
    search_query: String,
    command_query: String,
    chat_input: String,
    chat_messages: Vec<(bool, String)>,
    status: String,
    api_key: String,
    model: String,
    icons: HashMap<&'static str, egui::TextureHandle>,
    icons_loaded: bool,
    code_theme: egui_extras::syntax_highlighting::CodeTheme,
}

impl Default for MinuxIde {
    fn default() -> Self {
        let root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self {
            root,
            selected_file: None,
            editor_text: String::new(),
            dirty: false,
            sidebar_view: SidebarView::Explorer,
            settings_open: false,
            show_ai: false,
            show_output: false,
            search_query: String::new(),
            command_query: String::new(),
            chat_input: String::new(),
            chat_messages: vec![(false, "Я MINUX Agent. Интерфейс агента готов, но провайдер AI пока не подключён. Сейчас я могу подтвердить получение запроса.".into())],
            status: "Готово".into(),
            api_key: String::new(),
            model: "Не подключена".into(),
            icons: HashMap::new(),
            icons_loaded: false,
            code_theme: egui_extras::syntax_highlighting::CodeTheme::dark(13.5),
        }
    }
}

fn apply_theme(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(7.0, 5.0);
    style.spacing.button_padding = egui::vec2(9.0, 5.0);

    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = BG;
    visuals.window_fill = PANEL;
    visuals.extreme_bg_color = EDITOR_BG;
    visuals.faint_bg_color = PANEL_RAISED;
    visuals.code_bg_color = EDITOR_BG;
    visuals.hyperlink_color = ACCENT;
    visuals.override_text_color = Some(TEXT);
    visuals.selection.bg_fill = ACCENT_BG;
    visuals.selection.stroke = Stroke::new(1.0, ACCENT);
    visuals.widgets.noninteractive.bg_fill = PANEL;
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT);
    visuals.widgets.inactive.bg_fill = PANEL_RAISED;
    visuals.widgets.inactive.weak_bg_fill = PANEL_RAISED;
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, MUTED);
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER);
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(42, 48, 61);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, TEXT);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, Color32::from_rgb(67, 78, 100));
    visuals.widgets.active.bg_fill = ACCENT_BG;
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, TEXT);
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT);
    style.visuals = visuals;
    ctx.set_style(style);
}

fn load_icon_textures(ctx: &egui::Context) -> HashMap<&'static str, egui::TextureHandle> {
    let assets: [(&'static str, &'static str); 24] = [
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
    ];
    let mut icons = HashMap::with_capacity(assets.len() + 1);
    for (name, svg) in assets.into_iter().chain([("lang-yaml", include_str!("../assets/icons/lang-yaml.svg"))]) {
        match egui_extras::image::load_svg_bytes(svg.as_bytes()) {
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
    let (rect, response) = ui.allocate_exact_size(egui::vec2(34.0, 31.0), egui::Sense::click());
    if selected || response.hovered() {
        ui.painter().rect_filled(
            rect,
            egui::CornerRadius::same(5),
            if selected { ACCENT_BG } else { PANEL_RAISED },
        );
    }
    let icon_rect = egui::Rect::from_center_size(rect.center(), egui::vec2(18.0, 18.0));
    paint_icon_at(ui, icons, name, icon_rect, if selected { ACCENT } else { MUTED });
    response.on_hover_text(tooltip)
}

impl MinuxIde {
    fn open_workspace(&mut self) {
        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
            self.root = folder;
            self.selected_file = None;
            self.editor_text.clear();
            self.dirty = false;
            self.settings_open = false;
            self.status = "Рабочая папка открыта".into();
        }
    }

    fn open_file(&mut self, path: PathBuf) {
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

    fn draw_tree(&mut self, ui: &mut egui::Ui, dir: &Path, depth: usize) {
        if depth > 3 {
            return;
        }
        let Ok(entries) = fs::read_dir(dir) else {
            ui.label(RichText::new("Не удалось прочитать папку").color(MUTED).size(11.0));
            return;
        };

        let mut entries: Vec<_> = entries.filter_map(Result::ok).collect();
        entries.sort_by_key(|entry| (!entry.path().is_dir(), entry.file_name()));

        for entry in entries {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if path.file_name().and_then(|s| s.to_str()).is_some_and(|s| {
                s.starts_with('.') || matches!(s, "target" | "node_modules" | "bin" | "obj")
            }) {
                continue;
            }

            if path.is_dir() {
                let response = egui::CollapsingHeader::new(
                    RichText::new(format!("      {name}")).size(12.0).color(TEXT),
                )
                .id_salt(path.to_string_lossy().to_string())
                .default_open(depth == 0)
                .show(ui, |ui| self.draw_tree(ui, &path, depth + 1));
                let icon_rect = egui::Rect::from_center_size(
                    egui::pos2(response.header_response.rect.left() + 27.0, response.header_response.rect.center().y),
                    egui::vec2(14.0, 14.0),
                );
                paint_icon_at(ui, &self.icons, "folder", icon_rect, Color32::WHITE);
            } else {
                let selected = self.selected_file.as_ref() == Some(&path);
                ui.horizontal(|ui| {
                    ui.add_space(7.0);
                    draw_icon(ui, &self.icons, file_icon_key_for_path(&path), 16.0, Color32::WHITE);
                    if ui.selectable_label(selected, RichText::new(name.clone()).size(12.0).color(if selected { TEXT } else { MUTED })).clicked() {
                        self.open_file(path.clone());
                    }
                });
            }
        }
    }

    fn draw_sidebar(&mut self, ui: &mut egui::Ui) {
        ui.add_space(7.0);
        match self.sidebar_view {
            SidebarView::Explorer => {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("ПРОВОДНИК").size(10.0).strong().color(MUTED));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("↻").on_hover_text("Обновить дерево").clicked() {
                            self.status = "Дерево файлов обновлено".into();
                        }
                    });
                });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    draw_icon(ui, &self.icons, "folder-open", 16.0, Color32::WHITE);
                    ui.label(RichText::new(self.root.file_name().unwrap_or_default().to_string_lossy()).strong().size(12.0));
                });
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let root = self.root.clone();
                    self.draw_tree(ui, &root, 0);
                });
            }
            SidebarView::Search => {
                ui.horizontal(|ui| {
                    draw_icon(ui, &self.icons, "search", 15.0, MUTED);
                    ui.label(RichText::new("ПОИСК ФАЙЛОВ").size(10.0).strong().color(MUTED));
                });
                ui.add_space(8.0);
                ui.add(egui::TextEdit::singleline(&mut self.search_query).hint_text("Имя файла...").desired_width(f32::INFINITY));
                ui.add_space(8.0);
                if self.search_query.trim().is_empty() {
                    ui.label(RichText::new("Введите часть имени файла, чтобы найти его в проекте.").size(11.0).color(MUTED));
                } else {
                    let mut matches = Vec::new();
                    collect_search_matches(&self.root, &self.search_query, 0, &mut matches);
                    ui.label(RichText::new(format!("РЕЗУЛЬТАТЫ · {}", matches.len())).size(10.0).strong().color(MUTED));
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for path in matches {
                            let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                            ui.horizontal(|ui| {
                                draw_icon(ui, &self.icons, file_icon_key_for_path(&path), 16.0, Color32::WHITE);
                                if ui.selectable_label(false, RichText::new(name).size(12.0)).clicked() {
                                    self.open_file(path.clone());
                                }
                            });
                        }
                    });
                }
            }
            SidebarView::Extensions => {
                ui.label(RichText::new("ЯЗЫКИ И ГРАММАТИКИ").size(10.0).strong().color(MUTED));
                ui.add_space(8.0);
                ui.label(RichText::new("Подсветка Syntect · встроенные грамматики").size(10.0).color(MUTED));
                ui.separator();
                for (icon, label) in [
                    ("lang-rust", "Rust"),
                    ("lang-typescript", "TypeScript / TSX"),
                    ("lang-javascript", "JavaScript / JSX"),
                    ("lang-python", "Python"),
                    ("lang-c", "C"),
                    ("lang-cplusplus", "C / C++ headers"),
                    ("lang-csharp", "C#"),
                    ("lang-bash", "Shell / Bash"),
                    ("lang-xml", "XML / XSL / XSLT"),
                    ("lang-make", "Makefile / GNU Make"),
                    ("lang-html5", "HTML"),
                    ("lang-css3", "CSS"),
                    ("lang-json", "JSON"),
                    ("lang-yaml", "YAML"),
                ] {
                    ui.add_space(5.0);
                    ui.horizontal(|ui| {
                        draw_icon(ui, &self.icons, icon, 17.0, Color32::WHITE);
                        ui.label(RichText::new(label).size(11.0).color(TEXT));
                    });
                }
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
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("MINUX AGENT").strong().size(12.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("×").on_hover_text("Закрыть AI-панель").clicked() {
                    self.show_ai = false;
                }
            });
        });
        ui.horizontal(|ui| {
            ui.label(RichText::new("●").color(ORANGE).size(10.0));
            ui.label(RichText::new("Провайдер не подключён").size(10.0).color(MUTED));
        });
        ui.separator();

        let scroll_height = (ui.available_height() - 100.0).max(100.0);
        egui::ScrollArea::vertical()
            .max_height(scroll_height)
            .stick_to_bottom(true)
            .show(ui, |ui| {
                for (user, message) in &self.chat_messages {
                    ui.add_space(5.0);
                    ui.label(RichText::new(if *user { "ВЫ" } else { "AGENT" }).size(9.0).strong().color(if *user { ACCENT } else { GREEN }));
                    ui.add(egui::Label::new(RichText::new(message).size(12.0).color(TEXT)).wrap());
                    ui.add_space(8.0);
                    ui.separator();
                }
            });

        ui.add_space(6.0);
        let response = ui.add_sized(
            [ui.available_width(), 38.0],
            egui::TextEdit::singleline(&mut self.chat_input).hint_text("Спросите о проекте..."),
        );
        ui.add_space(5.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Enter — отправить").size(9.0).color(MUTED));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let send_clicked = ui.button("Отправить").clicked();
                let send_enter = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if (send_clicked || send_enter) && !self.chat_input.trim().is_empty() {
                    let prompt = self.chat_input.trim().to_string();
                    self.chat_messages.push((true, prompt.clone()));
                    let answer = run_csharp_agent(&prompt)
                        .unwrap_or_else(|error| format!("Ошибка C# Agent: {error}"));
                    self.chat_messages.push((false, answer));
                    self.chat_input.clear();
                }
            });
        });
    }

    fn draw_settings(&mut self, ui: &mut egui::Ui) {
        ui.add_space(20.0);
        ui.label(RichText::new("Настройки").size(26.0).strong().color(TEXT));
        ui.label(RichText::new("Внешний вид, рабочая среда и подключение AI.").size(12.0).color(MUTED));
        ui.add_space(20.0);
        ui.separator();
        ui.add_space(8.0);
        ui.label(RichText::new("AI-ПРОВАЙДЕР").size(10.0).strong().color(ACCENT));
        ui.add_space(8.0);
        ui.label(RichText::new("Название модели или провайдера").size(12.0).color(TEXT));
        ui.add_sized([360.0, 34.0], egui::TextEdit::singleline(&mut self.model).hint_text("Например, Groq"));
        ui.add_space(10.0);
        ui.label(RichText::new("API-ключ").size(12.0).color(TEXT));
        ui.add_sized([360.0, 34.0], egui::TextEdit::singleline(&mut self.api_key).password(true).hint_text("Введите API-ключ"));
        ui.add_space(6.0);
        ui.label(RichText::new("Ключ пока не сохраняется, запросы к API не выполняются.").size(11.0).color(MUTED));
        ui.add_space(24.0);
        ui.separator();
        ui.add_space(12.0);
        ui.label(RichText::new("КОМПОНЕНТЫ").size(10.0).strong().color(ACCENT));
        settings_row(ui, "UI / Core", "Rust · egui", GREEN);
        settings_row(ui, "Syntax highlighting", "egui_extras + Syntect", GREEN);
        settings_row(ui, "SVG rendering", "resvg", GREEN);
        settings_row(ui, "Icon assets", "Lucide + Devicon", GREEN);
        settings_row(ui, "Native Engine", &format!("C++ · v{}", unsafe { minux_engine_version() }), GREEN);
        settings_row(ui, "Agent", &format!("C# NativeAOT · v{}", agent_version()), GREEN);
        ui.add_space(8.0);
        ui.label(RichText::new("Языки: Rust, TS/TSX, JS/JSX, Python, C/C++, C#, Shell, XML/XSLT, Makefile, HTML, CSS, JSON, YAML.").size(11.0).color(MUTED));
        ui.add_space(16.0);
        if ui.button("← Вернуться в редактор").clicked() {
            self.settings_open = false;
        }
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

            let code_theme = self.code_theme.clone();
            let syntax = syntax_selector_for_path(&path).to_string();
            let mut layouter = move |ui: &egui::Ui, source: &str, wrap_width: f32| {
                let mut job = egui_extras::syntax_highlighting::highlight(
                    ui.ctx(),
                    ui.style(),
                    &code_theme,
                    source,
                    &syntax,
                );
                job.wrap.max_width = wrap_width;
                ui.fonts(|fonts| fonts.layout_job(job))
            };

            let available = ui.available_size();
            let response = ui.add_sized(
                [available.x.max(80.0), available.y.max(100.0)],
                egui::TextEdit::multiline(&mut self.editor_text)
                    .font(egui::TextStyle::Monospace)
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
        ui.label(RichText::new(format!("MINUX IDE  ›  {}", self.status)).size(11.0).color(MUTED));
        ui.label(RichText::new("Терминал и сборочный вывод ещё не подключены.").size(11.0).color(MUTED));
    }
}

impl eframe::App for MinuxIde {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.icons_loaded {
            self.icons = load_icon_textures(ctx);
            self.icons_loaded = true;
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

        egui::TopBottomPanel::top("main_toolbar")
            .exact_height(46.0)
            .frame(egui::Frame::new().fill(PANEL).stroke(Stroke::new(1.0, BORDER)))
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.add_space(4.0);
                    ui.label(RichText::new("M").size(17.0).strong().color(ACCENT));
                    ui.label(RichText::new("MINUX").size(13.0).strong().color(TEXT));
                    ui.label(RichText::new("IDE").size(10.0).color(MUTED));
                    ui.separator();
                    ui.label(RichText::new(self.root.file_name().unwrap_or_default().to_string_lossy()).size(11.0).color(MUTED));
                    ui.add_space(8.0);
                    let search_width = 300.0_f32.min((ui.available_width() * 0.38).max(180.0));
                    let command_search = ui.add_sized(
                        [search_width, 29.0],
                        egui::TextEdit::singleline(&mut self.command_query)
                            .hint_text("Поиск файлов  Ctrl+P"),
                    );
                    if command_search.changed() {
                        self.search_query = self.command_query.clone();
                        if !self.command_query.trim().is_empty() {
                            self.sidebar_view = SidebarView::Search;
                            self.settings_open = false;
                        }
                    }
                    if toolbar_icon_button(ui, &self.icons, "folder-open", false, "Открыть проект").clicked() {
                        self.open_workspace();
                    }
                    if toolbar_icon_button(ui, &self.icons, "save", false, "Сохранить файл").clicked() {
                        self.save_file();
                    }
                    if toolbar_icon_button(ui, &self.icons, "bot", self.show_ai, "AI Agent").clicked() {
                        self.show_ai = !self.show_ai;
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
                        ui.label(RichText::new(format!("C# {} · C++ {}", agent_version(), unsafe { minux_engine_version() })).size(10.0).color(TEXT));
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
            .frame(egui::Frame::new().fill(RAIL_BG).stroke(Stroke::new(1.0, BORDER)))
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
            .min_width(190.0)
            .max_width(340.0)
            .resizable(true)
            .frame(egui::Frame::new().fill(PANEL).stroke(Stroke::new(1.0, BORDER)).inner_margin(egui::Margin::same(10)))
            .show(ctx, |ui| self.draw_sidebar(ui));

        if self.show_ai {
            egui::SidePanel::right("ai_sidebar")
                .default_width(320.0)
                .min_width(270.0)
                .max_width(410.0)
                .resizable(true)
                .frame(egui::Frame::new().fill(PANEL).stroke(Stroke::new(1.0, BORDER)).inner_margin(egui::Margin::same(12)))
                .show(ctx, |ui| self.draw_ai_sidebar(ui));
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(EDITOR_BG).inner_margin(egui::Margin::same(12)))
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
    }
}

fn activity_button(
    ui: &mut egui::Ui,
    icons: &HashMap<&'static str, egui::TextureHandle>,
    icon_name: &str,
    selected: bool,
    tooltip: &str,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(40.0, 38.0), egui::Sense::click());
    if selected || response.hovered() {
        ui.painter().rect_filled(
            rect,
            egui::CornerRadius::same(5),
            if selected { ACCENT_BG } else { PANEL_RAISED },
        );
    }
    if selected {
        let marker = egui::Rect::from_min_size(rect.left_top(), egui::vec2(2.0, rect.height()));
        ui.painter().rect_filled(marker, egui::CornerRadius::same(1), ACCENT);
    }
    let icon_rect = egui::Rect::from_center_size(rect.center(), egui::vec2(20.0, 20.0));
    paint_icon_at(ui, icons, icon_name, icon_rect, if selected { ACCENT } else { MUTED });
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
        "xml" | "xsl" | "xslt" | "xsd" | "dtd" => "XML",
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
        _ => "Text",
    }
}

fn collect_search_matches(root: &Path, query: &str, depth: usize, hits: &mut Vec<PathBuf>) {
    if depth > 5 || hits.len() >= 80 {
        return;
    }
    let Ok(entries) = fs::read_dir(root) else { return; };
    let needle = query.to_lowercase();
    for entry in entries.filter_map(Result::ok) {
        if hits.len() >= 80 { break; }
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || matches!(name.as_str(), "target" | "node_modules" | "bin" | "obj") {
            continue;
        }
        if path.is_dir() {
            collect_search_matches(&path, query, depth + 1, hits);
        } else if name.to_lowercase().contains(&needle) {
            hits.push(path);
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
        Box::new(|cc| {
            apply_theme(&cc.egui_ctx);
            Ok(Box::new(MinuxIde::default()))
        }),
    )
}
