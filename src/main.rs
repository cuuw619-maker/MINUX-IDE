use eframe::egui;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(PartialEq, Clone, Copy)]
enum View {
    Editor,
    Chat,
    Settings,
}

struct MinuxIde {
    root: PathBuf,
    selected_file: Option<PathBuf>,
    editor_text: String,
    dirty: bool,
    view: View,
    chat_input: String,
    chat_messages: Vec<(bool, String)>,
    status: String,
    api_key: String,
    model: String,
}

impl Default for MinuxIde {
    fn default() -> Self {
        let root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self {
            root,
            selected_file: None,
            editor_text: String::new(),
            dirty: false,
            view: View::Editor,
            chat_input: String::new(),
            chat_messages: vec![(false, "Привет! Я MINUX Agent. Подключение модели пока не настроено. Сейчас интерфейс работает в режиме заглушки.".into())],
            status: "Готово".into(),
            api_key: String::new(),
            model: "Модель не подключена".into(),
        }
    }
}

impl MinuxIde {
    fn open_file(&mut self, path: PathBuf) {
        match fs::read_to_string(&path) {
            Ok(text) => {
                self.editor_text = text;
                self.selected_file = Some(path);
                self.dirty = false;
                self.view = View::Editor;
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
        if depth > 2 {
            return;
        }
        let Ok(entries) = fs::read_dir(dir) else {
            ui.label("Не удалось прочитать папку");
            return;
        };
        let mut entries: Vec<_> = entries.filter_map(Result::ok).collect();
        entries.sort_by_key(|e| (!e.path().is_dir(), e.file_name()));
        for entry in entries {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if path.file_name().and_then(|s| s.to_str()).is_some_and(|s| s.starts_with('.')) {
                continue;
            }
            if path.is_dir() {
                egui::CollapsingHeader::new(format!("📁 {name}"))
                    .id_salt(path.to_string_lossy().to_string())
                    .default_open(depth == 0)
                    .show(ui, |ui| self.draw_tree(ui, &path, depth + 1));
            } else {
                let selected = self.selected_file.as_ref() == Some(&path);
                if ui.selectable_label(selected, format!("📄 {name}")).clicked() {
                    self.open_file(path);
                }
            }
        }
    }

    fn draw_chat(&mut self, ui: &mut egui::Ui) {
        ui.heading("MINUX Agent");
        ui.label("Локальный интерфейс агента · провайдер пока не подключён");
        ui.separator();
        egui::ScrollArea::vertical().stick_to_bottom(true).show(ui, |ui| {
            for (user, message) in &self.chat_messages {
                ui.horizontal_wrapped(|ui| {
                    ui.strong(if *user { "Вы:" } else { "Agent:" });
                    ui.label(message);
                });
                ui.add_space(8.0);
            }
        });
        ui.separator();
        ui.horizontal(|ui| {
            let response = ui.add_sized(
                [ui.available_width() - 78.0, 36.0],
                egui::TextEdit::singleline(&mut self.chat_input).hint_text("Напишите сообщение..."),
            );
            let send = ui.button("Отправить").clicked()
                || (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));
            if send && !self.chat_input.trim().is_empty() {
                let prompt = self.chat_input.trim().to_string();
                self.chat_messages.push((true, prompt.clone()));
                self.chat_messages.push((false, format!(
                    "Это заглушка агента. Сообщение принято: «{}». Для реальных ответов нужно подключить API-провайдер и обработчик инструментов.",
                    prompt
                )));
                self.chat_input.clear();
            }
        });
    }
}

impl eframe::App for MinuxIde {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::S)) {
            self.save_file();
        }

        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("MINUX IDE");
                ui.separator();
                if ui.selectable_label(self.view == View::Editor, "Редактор").clicked() {
                    self.view = View::Editor;
                }
                if ui.selectable_label(self.view == View::Chat, "AI Agent").clicked() {
                    self.view = View::Chat;
                }
                if ui.selectable_label(self.view == View::Settings, "Настройки").clicked() {
                    self.view = View::Settings;
                }
                ui.separator();
                if ui.button("Открыть папку").clicked() {
                    if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                        self.root = folder;
                        self.selected_file = None;
                        self.editor_text.clear();
                        self.dirty = false;
                        self.status = "Папка проекта открыта".into();
                    }
                }
                if ui.button("Сохранить").clicked() {
                    self.save_file();
                }
            });
        });

        egui::SidePanel::left("project_files")
            .resizable(true)
            .default_width(235.0)
            .min_width(160.0)
            .show(ctx, |ui| {
                ui.heading("Проводник");
                ui.label(self.root.to_string_lossy());
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let root = self.root.clone();
                    self.draw_tree(ui, &root, 0);
                });
            });

        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(&self.status);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(if self.dirty { "Изменения не сохранены" } else { "Готово" });
                    ui.separator();
                    ui.label("Rust · MINUX Engine");
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| match self.view {
            View::Editor => {
                if let Some(path) = self.selected_file.clone() {
                    ui.horizontal(|ui| {
                        ui.heading(path.file_name().unwrap_or_default().to_string_lossy());
                        if self.dirty {
                            ui.label("●");
                        }
                    });
                    ui.label(path.to_string_lossy());
                    ui.separator();

                    let mut layouter = |ui: &egui::Ui, text: &dyn egui::TextBuffer, wrap_width: f32| {
                        let source = text.as_str();
                        let mut job = egui::text::LayoutJob::default();
                        for line in source.split_inclusive('\n') {
                            let trimmed = line.trim_start();
                            let color = if trimmed.starts_with("//") || trimmed.starts_with('#') {
                                egui::Color32::from_rgb(106, 165, 115)
                            } else if ["fn ", "pub ", "use ", "let ", "struct ", "impl ", "class ", "using ", "return ", "if ", "else", "import ", "def "]
                                .iter().any(|kw| trimmed.starts_with(kw)) {
                                egui::Color32::from_rgb(115, 160, 230)
                            } else if trimmed.contains("= ") || trimmed.contains("=>") {
                                egui::Color32::from_rgb(210, 180, 120)
                            } else {
                                ui.visuals().text_color()
                            };
                            job.append(line, 0.0, egui::TextFormat {
                                font_id: egui::FontId::monospace(14.0),
                                color,
                                ..Default::default()
                            });
                        }
                        job.wrap.max_width = wrap_width;
                        ui.fonts_mut(|f| f.layout_job(job))
                    };

                    let response = ui.add(
                        egui::TextEdit::multiline(&mut self.editor_text)
                            .font(egui::TextStyle::Monospace)
                            .desired_width(f32::INFINITY)
                            .desired_rows(30)
                            .code_editor()
                            .layouter(&mut layouter),
                    );
                    if response.changed() {
                        self.dirty = true;
                        self.status = "Есть несохранённые изменения".into();
                    }
                } else {
                    ui.vertical_centered(|ui| {
                        ui.add_space(100.0);
                        ui.heading("Добро пожаловать в MINUX IDE");
                        ui.label("Откройте папку проекта и выберите файл в проводнике.");
                        ui.label("Редактирование, сохранение и базовая подсветка синтаксиса доступны.");
                    });
                }
            }
            View::Chat => self.draw_chat(ui),
            View::Settings => {
                ui.heading("Настройки");
                ui.label("Базовая страница настроек. Сохранение параметров будет добавлено позже.");
                ui.separator();
                ui.label("AI-провайдер");
                ui.text_edit_singleline(&mut self.model);
                ui.label("API-ключ");
                ui.add(egui::TextEdit::singleline(&mut self.api_key).password(true).hint_text("Введите API-ключ"));
                ui.label("Пока значения не сохраняются и API-запросы не выполняются.");
                ui.add_space(12.0);
                ui.label("Ядро интерфейса: Rust + egui");
                ui.label("Планируемые подсистемы: C++ — нативные инструменты; C# — интеграции и расширения.");
            }
        });
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("MINUX IDE")
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([800.0, 520.0]),
        ..Default::default()
    };
    eframe::run_native(
        "MINUX IDE",
        options,
        Box::new(|_cc| Ok(Box::new(MinuxIde::default()))),
    )
}
