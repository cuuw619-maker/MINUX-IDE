use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    env, fs,
    path::{Component, Path, PathBuf},
    time::Duration,
};

const CHAT_URL: &str = "https://router.huggingface.co/v1/chat/completions";
const MODELS_URL: &str = "https://router.huggingface.co/v1/models";
const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;
const MAX_TOOL_ROUNDS: usize = 8;

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AiSettings {
    pub token: String,
    pub model: String,
    pub thinking_enabled: bool,
    pub max_tokens: u32,
    pub theme_accent: String,
    pub corner_radius: u8,
    pub animations_enabled: bool,
    pub editor_font_size: f32,
    pub recent_workspaces: Vec<String>,
    pub custom_accent: String,
    pub animation_speed: f32,
}

impl Default for AiSettings {
    fn default() -> Self {
        Self {
            token: String::new(),
            model: "Qwen/Qwen2.5-Coder-32B-Instruct".into(),
            thinking_enabled: true,
            max_tokens: 2048,
            theme_accent: "ocean".into(),
            corner_radius: 6,
            animations_enabled: true,
            editor_font_size: 13.5,
            recent_workspaces: Vec::new(),
            custom_accent: "#6E9BFF".into(),
            animation_speed: 1.0,
        }
    }
}

#[derive(Clone)]
pub struct AgentResponse {
    pub content: String,
    pub reasoning: Option<String>,
    pub history: Vec<Value>,
    pub workspace_changed: bool,
}

pub fn suggested_models() -> &'static [&'static str] {
    &[
        "Qwen/Qwen2.5-Coder-32B-Instruct",
        "Qwen/Qwen3-8B",
        "deepseek-ai/DeepSeek-R1",
        "meta-llama/Llama-3.3-70B-Instruct",
        "mistralai/Mistral-7B-Instruct-v0.3",
    ]
}

fn config_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    let base = env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    #[cfg(target_os = "macos")]
    let base = env::var_os("HOME")
        .map(|home| PathBuf::from(home).join("Library").join("Application Support"))
        .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    let base = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    base.join("MINUX-IDE").join("settings.json")
}

pub fn load_settings() -> AiSettings {
    let mut settings: AiSettings = fs::read(config_path())
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default();

    settings.model = normalize_model_id(&settings.model)
        .unwrap_or_else(|_| AiSettings::default().model);
    settings.corner_radius = settings.corner_radius.clamp(3, 14);
    settings.editor_font_size = settings.editor_font_size.clamp(11.0, 20.0);
    settings.animation_speed = if settings.animation_speed.is_finite() {
        settings.animation_speed.clamp(0.5, 2.0)
    } else {
        1.0
    };
    settings.custom_accent = normalize_hex_color(&settings.custom_accent);
    settings.recent_workspaces.truncate(8);
    settings
}

unsafe extern "C" {
    fn minux_model_id_is_valid(data: *const u8, length: usize) -> i32;
}

fn normalize_hex_color(input: &str) -> String {
    let value = input.trim();
    if value.len() == 7
        && value.starts_with('#')
        && value[1..].bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        format!("#{}", value[1..].to_ascii_uppercase())
    } else {
        "#6E9BFF".to_owned()
    }
}

fn c_model_id_is_valid(value: &str) -> bool {
    // SAFETY: C reads only the byte range passed in and does not retain its pointer.
    unsafe { minux_model_id_is_valid(value.as_ptr(), value.len()) == 1 }
}

pub fn normalize_model_id(input: &str) -> Result<String, String> {
    let value = input.trim().trim_matches(|c| c == '\'' || c == '"' || c == '`').trim();
    if c_model_id_is_valid(value) {
        return Ok(value.to_owned());
    }

    // Older settings could append the default model ID to a user selection.
    // Preserve the valid model prefix before that known default suffix.
    for candidate in suggested_models() {
        if let Some(prefix) = value.strip_suffix(candidate) {
            let prefix = prefix.trim();
            if c_model_id_is_valid(prefix) {
                return Ok(prefix.to_owned());
            }
        }
    }

    let matches: Vec<&str> = suggested_models()
        .iter()
        .copied()
        .filter(|candidate| value.contains(candidate))
        .collect();
    if matches.len() == 1 {
        return Ok(matches[0].to_owned());
    }

    Err("Некорректный Hugging Face model ID. Выбери модель из списка или используй формат owner/model.".into())
}

pub fn save_settings(settings: &AiSettings) -> Result<(), String> {
    let path = config_path();
    let parent = path.parent().ok_or_else(|| "Не удалось определить каталог настроек.".to_string())?;
    fs::create_dir_all(parent).map_err(|e| format!("Не удалось создать каталог настроек: {e}"))?;
    let data = serde_json::to_vec_pretty(settings)
        .map_err(|e| format!("Не удалось сериализовать настройки: {e}"))?;
    fs::write(&path, data).map_err(|e| format!("Не удалось сохранить настройки: {e}"))
}

fn build_client() -> Result<Client, String> {
    Client::builder()
        .timeout(Duration::from_secs(120))
        .connect_timeout(Duration::from_secs(15))
        .user_agent(concat!("MINUX-IDE/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| format!("Не удалось инициализировать HTTP-клиент: {e}"))
}

pub fn fetch_models(token: &str) -> Result<Vec<String>, String> {
    if token.trim().is_empty() {
        return Err("Сначала укажи Hugging Face Access Token.".into());
    }

    let client = build_client()?;
    let response = client.get(MODELS_URL).bearer_auth(token.trim()).send()
        .map_err(|e| format!("Не удалось подключиться к Hugging Face: {e}"))?;
    let status = response.status();
    let body = response.text().map_err(|e| format!("Не удалось прочитать ответ Hugging Face: {e}"))?;

    if !status.is_success() {
        return Err(format_hf_error(status.as_u16(), &body));
    }

    let json: Value = serde_json::from_str(&body)
        .map_err(|e| format!("Hugging Face вернул неожиданный список моделей: {e}"))?;
    let items = json.get("data").and_then(Value::as_array)
        .ok_or_else(|| "Ответ Hugging Face не содержит списка моделей.".to_string())?;

    let mut models: Vec<String> = items.iter()
        .filter_map(|item| item.get("id").and_then(Value::as_str).map(str::to_owned))
        .filter(|id| !id.trim().is_empty())
        .collect();
    models.sort_unstable();
    models.dedup();
    // Keep the model picker responsive when the account exposes a very large catalogue.
    models.truncate(300);
    if models.is_empty() {
        return Err("Для этого токена API не вернул доступных моделей. Можно указать model ID вручную.".into());
    }
    Ok(models)
}

fn format_hf_error(status: u16, body: &str) -> String {
    let parsed = serde_json::from_str::<Value>(body).ok();
    if status == 400
        && parsed.as_ref()
            .and_then(|value| value.pointer("/error/code").or_else(|| value.get("code")))
            .and_then(Value::as_str) == Some("model_not_found")
    {
        let message = parsed.as_ref()
            .and_then(|value| value.pointer("/error/message").or_else(|| value.get("message")))
            .and_then(Value::as_str)
            .unwrap_or("Модель не существует или недоступна этому endpoint.");
        return format!("MINUX Agent: Hugging Face не нашёл модель. Проверь точный ID и доступность Inference Provider. Открой настройки и выбери точную модель из списка доступных. {message}");
    }

    let detail = parsed
        .and_then(|v| v.get("error").cloned())
        .and_then(|v| v.as_str().map(str::to_owned).or_else(|| Some(v.to_string())))
        .unwrap_or_else(|| body.to_owned());
    let detail = detail.trim();
    let detail = if detail.len() > 900 {
        let mut end = 900;
        while !detail.is_char_boundary(end) {
            end -= 1;
        }
        &detail[..end]
    } else {
        detail
    };
    match status {
        401 => format!("Hugging Face отклонил токен (401). Проверь Access Token и его разрешения. {detail}"),
        403 => format!("Доступ запрещён (403). Проверь разрешения токена и доступ к модели. {detail}"),
        404 => format!("Модель или endpoint не найдены (404). Проверь model ID и доступность провайдера. {detail}"),
        429 => format!("Лимит Hugging Face превышен (429). Подожди или проверь квоты. {detail}"),
        402 => format!("Провайдер требует доступный баланс/квоту (402). {detail}"),
        _ => format!("Ошибка Hugging Face (HTTP {status}). {detail}"),
    }
}

pub fn run_agent(
    settings: AiSettings,
    history: Vec<Value>,
    workspace_root: PathBuf,
) -> Result<AgentResponse, String> {
    if settings.token.trim().is_empty() {
        return Err("Открой Настройки → Hugging Face и укажи Access Token.".into());
    }
    if settings.model.trim().is_empty() {
        return Err("Укажи ID модели Hugging Face.".into());
    }
    let mut settings = settings;
    settings.model = normalize_model_id(&settings.model)?;

    let root = workspace_root.canonicalize()
        .map_err(|e| format!("Не удалось открыть рабочую папку: {e}"))?;
    let client = build_client()?;

    let system = format!(
        "You are MINUX Agent, an AI coding assistant inside a local desktop IDE.\n        The user workspace root is: {}\n        Inspect project files with list_project_files and search_project before making changes. Read each existing file before editing it. Prefer replace_in_file for a unique targeted change; use write_file only when replacing the complete intended file.\n        You can inspect and change project files with the provided tools. Use tools instead of claiming an operation is done.\n        All paths passed to tools must be relative to the workspace. Never access paths outside the workspace.\n        Never read or write secret files such as .env, private keys, credentials, or secret stores.\n        Preserve unrelated code and make minimal targeted changes. For new project structures, create directories and files through tools. Never delete files or execute shell commands.\n        When done, summarize concrete files changed and any checks that were or were not run.\n        Answer in the user's language. Do not invent tool results.",
        root.display()
    );

    let mut messages: Vec<Value> = Vec::with_capacity(history.len() + 1);
    messages.push(json!({"role":"system","content":system}));
    let start = if history.len() > 36 {
        history.iter().enumerate().skip(history.len() - 36)
            .find(|(_, item)| item.get("role").and_then(Value::as_str) == Some("user"))
            .map(|(index, _)| index)
            .unwrap_or(history.len() - 36)
    } else {
        0
    };
    for item in history.into_iter().skip(start) {
        let role = item.get("role").and_then(Value::as_str).unwrap_or("");
        if matches!(role, "user" | "assistant" | "tool") {
            messages.push(item);
        }
    }

    let tools = tool_definitions();
    let mut latest_reasoning: Option<String> = None;
    let mut workspace_changed = false;

    for _round in 0..MAX_TOOL_ROUNDS {
        let message = request_assistant(&client, &settings, &messages, &tools)?;
        if let Some(reasoning) = extract_text(message.get("reasoning_content")
            .or_else(|| message.get("reasoning"))) {
            if !reasoning.trim().is_empty() {
                latest_reasoning = Some(reasoning);
            }
        }

        let calls = message.get("tool_calls").and_then(Value::as_array).cloned().unwrap_or_default();
        if !calls.is_empty() {
            let mut assistant_message = json!({"role":"assistant"});
            if let Some(content) = message.get("content") {
                assistant_message["content"] = content.clone();
            } else {
                assistant_message["content"] = Value::Null;
            }
            assistant_message["tool_calls"] = Value::Array(calls.clone());
            messages.push(assistant_message);

            for call in calls.iter().take(8) {
                let call_id = call.get("id").and_then(Value::as_str).unwrap_or("minux-tool-call");
                let function = call.get("function").cloned().unwrap_or(Value::Null);
                let name = function.get("name").and_then(Value::as_str).unwrap_or("");
                let argument_value = function.get("arguments").cloned().unwrap_or_else(|| json!({}));
                let arguments = match argument_value {
                    Value::String(text) => serde_json::from_str::<Value>(&text).unwrap_or_else(|_| json!({})),
                    value => value,
                };
                let outcome = execute_tool(name, &arguments, &root);
                if matches!(name, "write_file" | "create_file" | "create_directory" | "replace_in_file")
                    && !outcome.starts_with("Ошибка инструмента:")
                {
                    workspace_changed = true;
                }
                messages.push(json!({
                    "role":"tool",
                    "tool_call_id":call_id,
                    "content":outcome
                }));
            }
            continue;
        }

        let raw_content = extract_text(message.get("content")).unwrap_or_default();
        let (content, embedded_reasoning) = split_thinking_tags(&raw_content);
        if latest_reasoning.is_none() {
            latest_reasoning = embedded_reasoning;
        }

        if content.trim().is_empty() {
            return Err("Модель вернула пустой ответ. Попробуй другую модель или отключи режим thinking.".into());
        }

        messages.push(json!({"role":"assistant","content":raw_content}));
        let conversation = messages.into_iter().skip(1).collect();
        return Ok(AgentResponse {
            content,
            reasoning: latest_reasoning,
            history: conversation,
            workspace_changed,
        });
    }

    Err("Агент достиг лимита из 8 циклов вызова инструментов. Попроси выполнить задачу меньшими шагами.".into())
}

fn request_assistant(
    client: &Client,
    settings: &AiSettings,
    messages: &[Value],
    tools: &Value,
) -> Result<Value, String> {
    let mut active_messages = messages.to_vec();
    let mut send_thinking = settings.thinking_enabled;
    let mut send_tools = true;

    for _attempt in 0..5 {
        let mut payload = json!({
            "model": settings.model.as_str(),
            "messages": active_messages.clone(),
            "max_tokens": settings.max_tokens.clamp(256, 8192),
            "temperature": 0.2,
            "stream": false
        });
        if send_tools {
            payload["tools"] = tools.clone();
            payload["tool_choice"] = json!("auto");
        }
        if send_thinking {
            payload["chat_template_kwargs"] = json!({"enable_thinking": true});
        }

        let response = client.post(CHAT_URL)
            .bearer_auth(settings.token.trim())
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .map_err(|e| format!("Запрос к Hugging Face не выполнен: {e}"))?;

        let status = response.status();
        let body = response.text().map_err(|e| format!("Не удалось прочитать ответ модели: {e}"))?;
        if status.is_success() {
            let value: Value = serde_json::from_str(&body)
                .map_err(|e| format!("Ответ модели не является JSON: {e}"))?;
            return parse_assistant_message(&value);
        }

        let lower = body.to_ascii_lowercase();
        if matches!(status.as_u16(), 429 | 500 | 502 | 503 | 504) && _attempt < 2 {
            let delay_ms = if status.as_u16() == 429 {
                900 + (_attempt as u64 * 700)
            } else {
                300 * (1_u64 << _attempt)
            };
            std::thread::sleep(Duration::from_millis(delay_ms));
            continue;
        }
        if matches!(status.as_u16(), 400 | 422)
            && send_thinking
            && (lower.contains("chat_template_kwargs")
                || lower.contains("unknown parameter")
                || lower.contains("unsupported parameter"))
        {
            send_thinking = false;
            continue;
        }

        if matches!(status.as_u16(), 400 | 422)
            && send_tools
            && (lower.contains("tool_choice")
                || lower.contains("tool_calls")
                || lower.contains("tool calling")
                || lower.contains("tools are not supported")
                || lower.contains("function calling"))
        {
            send_tools = false;
            if let Some(system) = active_messages.first_mut() {
                let old_content = system.get("content").and_then(Value::as_str).unwrap_or("");
                system["content"] = json!(format!(
                    "{old_content}\nNOTE: This model/provider rejected function calling. Do not claim file changes. Give a plain-text answer and explain that file tools are unavailable for this model."
                ));
            }
            continue;
        }

        return Err(format_hf_error(status.as_u16(), &body));
    }

    Err("Модель не приняла параметры thinking/tools после повторных попыток. Выбери другую модель или отключи thinking.".into())
}

fn parse_assistant_message(response: &Value) -> Result<Value, String> {
    let message = response.get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.get("message"))
        .ok_or_else(|| format!("Ответ модели не содержит choices[0].message: {}", response.to_string().chars().take(500).collect::<String>()))?;
    Ok(message.clone())
}

fn extract_text(value: Option<&Value>) -> Option<String> {
    let value = value?;
    if let Some(text) = value.as_str() {
        return Some(text.to_owned());
    }
    if let Some(parts) = value.as_array() {
        let mut output = String::new();
        for part in parts {
            if let Some(text) = part.get("text").and_then(Value::as_str) {
                output.push_str(text);
            }
        }
        return Some(output);
    }
    None
}

fn split_thinking_tags(input: &str) -> (String, Option<String>) {
    let open = "<think>";
    let close = "</think>";
    let Some(start) = input.find(open) else {
        return (input.to_owned(), None);
    };
    let content_start = start + open.len();
    let Some(end_rel) = input[content_start..].find(close) else {
        return (input.to_owned(), None);
    };
    let end = content_start + end_rel;
    let reasoning = input[content_start..end].trim().to_owned();
    let mut content = String::with_capacity(input.len());
    content.push_str(input[..start].trim_end());
    content.push('\n');
    content.push_str(input[end + close.len()..].trim_start());
    (content.trim().to_owned(), if reasoning.is_empty() { None } else { Some(reasoning) })
}

fn tool_definitions() -> Value {
    json!([
        {"type":"function","function":{
            "name":"list_project_files",
            "description":"List files and directories in a directory inside the current workspace. Use path='.' for the root.",
            "parameters":{"type":"object","properties":{"path":{"type":"string","description":"Relative directory path, default '.'"}}}
        }},
        {"type":"function","function":{
            "name":"read_file",
            "description":"Read a UTF-8 text file inside the workspace before editing it.",
            "parameters":{"type":"object","properties":{"path":{"type":"string"}},"required":["path"]}
        }},
        {"type":"function","function":{
            "name":"create_directory",
            "description":"Create a directory tree inside the workspace.",
            "parameters":{"type":"object","properties":{"path":{"type":"string"}},"required":["path"]}
        }},
        {"type":"function","function":{
            "name":"create_file",
            "description":"Create a new UTF-8 text file. Fails rather than overwriting an existing file.",
            "parameters":{"type":"object","properties":{"path":{"type":"string"},"content":{"type":"string"}},"required":["path","content"]}
        }},
        {"type":"function","function":{
            "name":"write_file",
            "description":"Write or replace a UTF-8 text file inside the workspace. Read existing files first and preserve unrelated content.",
            "parameters":{"type":"object","properties":{"path":{"type":"string"},"content":{"type":"string"}},"required":["path","content"]}
        }},
        {"type":"function","function":{
            "name":"search_project",
            "description":"Search case-insensitive text across UTF-8 source files in the workspace. Use this before editing to find exact symbols and references.",
            "parameters":{"type":"object","properties":{"query":{"type":"string","description":"Text or symbol to find"},"path":{"type":"string","description":"Optional relative directory, defaults to '.'"}},"required":["query"]}
        }},
        {"type":"function","function":{
            "name":"replace_in_file",
            "description":"Replace an exact text fragment in one file only when it occurs exactly once. Use read_file first and preserve unrelated content.",
            "parameters":{"type":"object","properties":{"path":{"type":"string"},"search":{"type":"string","description":"Exact old text; must occur once"},"replacement":{"type":"string","description":"New text"}},"required":["path","search","replacement"]}
        }}
    ])
}

fn execute_tool(name: &str, arguments: &Value, root: &Path) -> String {
    let result = match name {
        "list_project_files" => {
            let rel = arguments.get("path").and_then(Value::as_str).unwrap_or(".");
            list_project_files(root, rel)
        }
        "read_file" => {
            let rel = arguments.get("path").and_then(Value::as_str).unwrap_or("");
            read_workspace_file(root, rel)
        }
        "create_directory" => {
            let rel = arguments.get("path").and_then(Value::as_str).unwrap_or("");
            create_workspace_directory(root, rel)
        }
        "create_file" => {
            let rel = arguments.get("path").and_then(Value::as_str).unwrap_or("");
            let content = arguments.get("content").and_then(Value::as_str).unwrap_or("");
            create_workspace_file(root, rel, content)
        }
        "write_file" => {
            let rel = arguments.get("path").and_then(Value::as_str).unwrap_or("");
            let content = arguments.get("content").and_then(Value::as_str).unwrap_or("");
            write_workspace_file(root, rel, content)
        }
        "search_project" => {
            let query = arguments.get("query").and_then(Value::as_str).unwrap_or("");
            let rel = arguments.get("path").and_then(Value::as_str).unwrap_or(".");
            search_workspace_files(root, rel, query)
        }
        "replace_in_file" => {
            let rel = arguments.get("path").and_then(Value::as_str).unwrap_or("");
            let search = arguments.get("search").and_then(Value::as_str).unwrap_or("");
            let replacement = arguments.get("replacement").and_then(Value::as_str).unwrap_or("");
            replace_workspace_text(root, rel, search, replacement)
        }
        _ => Err(format!("Инструмент '{name}' не существует.")),
    };
    match result {
        Ok(message) => message,
        Err(message) => format!("Ошибка инструмента: {message}"),
    }
}

fn resolve_workspace_path(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let relative_path = Path::new(relative);
    if relative_path.is_absolute() {
        return Err("Абсолютные пути запрещены. Используй путь относительно рабочей папки.".into());
    }
    if relative_path.components().any(|component| {
        matches!(component, Component::ParentDir | Component::RootDir | Component::Prefix(_))
    }) {
        return Err("Путь не должен выходить из рабочей папки (сегменты '..' запрещены).".into());
    }

    let mut result = root.to_path_buf();
    for component in relative_path.components() {
        if let Component::Normal(part) = component {
            result.push(part);
            let component_name = part.to_string_lossy().to_ascii_lowercase();
            if matches!(component_name.as_str(), ".git" | "target" | "node_modules") {
                return Err("Запись во внутренние/генерируемые каталоги запрещена.".into());
            }
            let extension = Path::new(component_name.as_str())
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("");
            let is_env_secret = component_name == ".env"
                || (component_name.starts_with(".env.") && component_name != ".env.example");
            let is_private_key = matches!(component_name.as_str(),
                "id_rsa" | "id_ed25519" | "id_ecdsa" | "credentials.json"
                | "secrets.json" | "authorized_keys" | "known_hosts"
            ) || matches!(extension, "key" | "pem" | "der" | "p12" | "pfx" | "keystore");
            if is_env_secret || is_private_key || component_name == ".ssh" || component_name == "secrets" {
                return Err("Доступ к файлам с потенциальными секретами запрещён инструментам агента.".into());
            }
        }
    }

    let mut ancestor = result.clone();
    while !ancestor.exists() {
        if !ancestor.pop() {
            return Err("Не удалось проверить путь.".into());
        }
    }
    let canonical_ancestor = ancestor.canonicalize()
        .map_err(|e| format!("Не удалось проверить каталог: {e}"))?;
    if !canonical_ancestor.starts_with(root) {
        return Err("Ссылка ведёт за пределы рабочей папки.".into());
    }

    if result.exists() {
        let canonical_target = result.canonicalize()
            .map_err(|e| format!("Не удалось проверить путь: {e}"))?;
        if !canonical_target.starts_with(root) {
            return Err("Путь ведёт за пределы рабочей папки.".into());
        }
    }
    Ok(result)
}

fn list_project_files(root: &Path, relative: &str) -> Result<String, String> {
    let path = resolve_workspace_path(root, relative)?;
    if !path.is_dir() {
        return Err(format!("'{}' не является каталогом.", relative));
    }
    let mut entries: Vec<_> = fs::read_dir(&path)
        .map_err(|e| format!("Не удалось прочитать каталог: {e}"))?
        .filter_map(Result::ok)
        .collect();
    entries.sort_by_key(|entry| (!entry.path().is_dir(), entry.file_name()));

    let mut lines = Vec::new();
    for entry in entries.into_iter().take(200) {
        let name = entry.file_name().to_string_lossy().to_string();
        if entry.path().is_dir() {
            lines.push(format!("{name}/"));
        } else {
            lines.push(name);
        }
    }
    if lines.is_empty() {
        Ok("(каталог пуст)".into())
    } else {
        Ok(lines.join("\n"))
    }
}

fn search_workspace_files(root: &Path, relative: &str, query: &str) -> Result<String, String> {
    let needle = query.trim().to_lowercase();
    if needle.is_empty() {
        return Err("Укажи текст или имя символа для поиска.".into());
    }
    if needle.len() > 200 {
        return Err("Строка поиска слишком длинная (максимум 200 байт).".into());
    }

    let start = resolve_workspace_path(root, relative)?;
    if !start.is_dir() {
        return Err(format!("'{}' не является каталогом.", relative));
    }

    let mut stack: Vec<(PathBuf, usize)> = vec![(start, 0)];
    let mut visited_entries = 0usize;
    let mut matches: Vec<String> = Vec::new();
    let mut truncated = false;

    while let Some((directory, depth)) = stack.pop() {
        let entries = match fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(_) => continue,
        };
        for entry in entries.filter_map(Result::ok) {
            visited_entries += 1;
            if visited_entries > 4000 || matches.len() >= 120 {
                truncated = true;
                break;
            }

            let item_path = entry.path();
            let kind = match entry.file_type() {
                Ok(kind) => kind,
                Err(_) => continue,
            };
            if kind.is_symlink() {
                continue;
            }

            let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
            if kind.is_dir() {
                if depth < 16 && !matches!(name.as_str(),
                    ".git" | "target" | "node_modules" | "dist" | "build" | "bin" | "obj" | ".venv" | "venv"
                ) {
                    stack.push((item_path, depth + 1));
                }
                continue;
            }
            if !kind.is_file() {
                continue;
            }

            let relative_path = match item_path.strip_prefix(root) {
                Ok(path) => path.to_string_lossy().replace('\\', "/"),
                Err(_) => continue,
            };
            if resolve_workspace_path(root, &relative_path).is_err() {
                continue;
            }
            match fs::metadata(&item_path) {
                Ok(metadata) if metadata.len() <= 512 * 1024 => {}
                _ => continue,
            }
            let contents = match fs::read_to_string(&item_path) {
                Ok(contents) => contents,
                Err(_) => continue,
            };
            for (line_number, line) in contents.lines().enumerate() {
                if line.to_lowercase().contains(&needle) {
                    let snippet: String = line.chars().take(240).collect();
                    matches.push(format!("{relative_path}:{}: {snippet}", line_number + 1));
                    if matches.len() >= 120 {
                        truncated = true;
                        break;
                    }
                }
            }
            if truncated {
                break;
            }
        }
        if truncated {
            break;
        }
    }

    if matches.is_empty() {
        Ok("Совпадений не найдено.".into())
    } else {
        let mut output = matches.join("\n");
        if output.len() > 28 * 1024 {
            let mut end = 28 * 1024;
            while !output.is_char_boundary(end) { end -= 1; }
            output.truncate(end);
            truncated = true;
        }
        if truncated {
            output.push_str("\n… поиск ограничен лимитом 4000 элементов / 120 совпадений.");
        }
        Ok(output)
    }
}

fn replace_workspace_text(root: &Path, relative: &str, search: &str, replacement: &str) -> Result<String, String> {
    if search.len() > 128 * 1024 || replacement.len() > MAX_FILE_BYTES as usize {
        return Err("Фрагмент или замена превышает лимит размера.".into());
    }
    let path = resolve_workspace_path(root, relative)?;
    let contents = read_workspace_file(root, relative)?;
    let updated = replace_exactly_once(&contents, search, replacement)?;
    write_workspace_file(root, relative, &updated)?;
    Ok(format!("Заменён один фрагмент в {}.", path.strip_prefix(root).unwrap_or(&path).display()))
}

fn replace_exactly_once(contents: &str, search: &str, replacement: &str) -> Result<String, String> {
    if search.is_empty() {
        return Err("Фрагмент для замены не может быть пустым.".into());
    }
    let occurrences = contents.match_indices(search).take(2).count();
    match occurrences {
        0 => Err("Точный фрагмент не найден. Обнови чтение файла и попробуй снова.".into()),
        1 => Ok(contents.replacen(search, replacement, 1)),
        _ => Err("Фрагмент встречается несколько раз. Уточни текст для однозначной замены.".into()),
    }
}

fn read_workspace_file(root: &Path, relative: &str) -> Result<String, String> {
    let path = resolve_workspace_path(root, relative)?;
    let metadata = fs::metadata(&path).map_err(|e| format!("Не удалось прочитать метаданные: {e}"))?;
    if !metadata.is_file() {
        return Err("Путь не является файлом.".into());
    }
    if metadata.len() > MAX_FILE_BYTES {
        return Err("Файл больше 2 MiB. Для безопасности агент не читает его целиком.".into());
    }
    fs::read_to_string(path).map_err(|e| format!("Не удалось прочитать UTF-8 файл: {e}"))
}

fn create_workspace_directory(root: &Path, relative: &str) -> Result<String, String> {
    if relative.trim().is_empty() || relative.trim() == "." {
        return Err("Укажи имя нового каталога внутри проекта.".into());
    }
    let path = resolve_workspace_path(root, relative)?;
    fs::create_dir_all(&path).map_err(|e| format!("Не удалось создать каталог: {e}"))?;
    let canonical_target = path.canonicalize().map_err(|e| format!("Не удалось проверить каталог: {e}"))?;
    if !canonical_target.starts_with(root) {
        return Err("Путь после создания оказался вне рабочей папки.".into());
    }
    Ok(format!("Каталог создан: {}", relative))
}

fn create_workspace_file(root: &Path, relative: &str, content: &str) -> Result<String, String> {
    let path = resolve_workspace_path(root, relative)?;
    if path.exists() {
        return Err(format!("Файл '{}' уже существует. Прочитай его и используй write_file для изменения.", relative));
    }
    write_workspace_file(root, relative, content)
}

fn write_workspace_file(root: &Path, relative: &str, content: &str) -> Result<String, String> {
    if content.len() > MAX_FILE_BYTES as usize {
        return Err("Содержимое больше 2 MiB. Раздели его на несколько файлов.".into());
    }
    let path = resolve_workspace_path(root, relative)?;
    let parent = path.parent().ok_or_else(|| "Не удалось определить родительский каталог.".to_string())?;
    fs::create_dir_all(parent).map_err(|e| format!("Не удалось создать родительский каталог: {e}"))?;
    let path = resolve_workspace_path(root, relative)?;
    fs::write(&path, content).map_err(|e| format!("Не удалось записать файл: {e}"))?;
    Ok(format!("Файл сохранён: {}", relative))
}

#[cfg(test)]
mod model_id_tests {
    use super::{normalize_hex_color, normalize_model_id, replace_exactly_once};

    #[test]
    fn repairs_concatenated_legacy_model_id() {
        assert_eq!(
            normalize_model_id("zai-org/GLM-5.3Qwen/Qwen2.5-Coder-32B-Instruct").unwrap(),
            "zai-org/GLM-5.3"
        );
    }

    #[test]
    fn keeps_well_formed_model_ids() {
        assert_eq!(normalize_model_id("zai-org/GLM-5.3").unwrap(), "zai-org/GLM-5.3");
    }

    #[test]
    fn rejects_unknown_multi_part_ids() {
        assert!(normalize_model_id("owner/model/second").is_err());
    }

    #[test]
    fn normalizes_custom_hex_colors() {
        assert_eq!(normalize_hex_color("#ab12Ef"), "#AB12EF");
        assert_eq!(normalize_hex_color("invalid"), "#6E9BFF");
    }

    #[test]
    fn exact_replacement_requires_one_match() {
        assert_eq!(replace_exactly_once("const a = 1;", "a = 1", "a = 2").unwrap(), "const a = 2;");
        assert!(replace_exactly_once("abc", "", "x").is_err());
        assert!(replace_exactly_once("abc abc", "abc", "x").is_err());
        assert!(replace_exactly_once("abc", "missing", "x").is_err());
    }
}
