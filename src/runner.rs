use std::{
    fs,
    path::Path,
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

const MAX_OUTPUT_BYTES: usize = 48 * 1024;

/// Runs only the file explicitly selected by the user. Commands are passed as
/// argument arrays (never through a shell) and every target must stay in the workspace.
pub fn run_file(root: &Path, selected: &Path) -> Result<String, String> {
    let root = root.canonicalize().map_err(|e| format!("Не удалось открыть проект: {e}"))?;
    let selected = selected.canonicalize().map_err(|e| format!("Не удалось открыть файл: {e}"))?;
    if !selected.starts_with(&root) {
        return Err("Запуск за пределами рабочей папки запрещён.".into());
    }
    if !selected.is_file() {
        return Err("Выбранный путь не является файлом.".into());
    }
    if fs::metadata(&selected).map(|meta| meta.len() > 4 * 1024 * 1024).unwrap_or(true) {
        return Err("Запуск остановлен: файл отсутствует или превышает 4 MiB.".into());
    }

    let extension = selected.extension().and_then(|item| item.to_str()).unwrap_or("").to_ascii_lowercase();
    let name = selected.file_name().and_then(|item| item.to_str()).unwrap_or("").to_ascii_lowercase();

    match extension.as_str() {
        "js" | "mjs" | "cjs" => run_program("node", &[selected.as_os_str()], &root),
        "jsx" => run_typescript(&selected, &root, true),
        "ts" => run_typescript(&selected, &root, false),
        "tsx" => run_typescript(&selected, &root, true),
        "kt" => run_kotlin_source(&selected, &root),
        "kts" => run_program("kotlinc", &[std::ffi::OsStr::new("-script"), selected.as_os_str()], &root),
        "py" | "pyw" => run_python(&selected, &root),
        "sh" | "bash" => run_fallback("bash", &[selected.as_os_str()], "sh", &[selected.as_os_str()], &root),
        "ps1" => run_fallback("pwsh", &[std::ffi::OsStr::new("-NoProfile"), std::ffi::OsStr::new("-File"), selected.as_os_str()],
            "powershell", &[std::ffi::OsStr::new("-NoProfile"), std::ffi::OsStr::new("-File"), selected.as_os_str()], &root),
        "c" => run_c(&selected, &root),
        "xsl" | "xslt" => run_xslt(&selected, &root),
        _ if matches!(name.as_str(), "makefile" | "gnumakefile") || extension == "mk" || extension == "make" => {
            run_make(&selected, &root)
        }
        _ => Err(format!(
            "Для .{} пока нет команды запуска. Поддерживаются TypeScript/JavaScript, Shell, Python, C, Make и XSLT.",
            if extension.is_empty() { "unknown" } else { &extension }
        )),
    }
}

fn run_typescript(file: &Path, root: &Path, jsx: bool) -> Result<String, String> {
    if jsx {
        return run_with_tsx(file, root);
    }
    let args = [std::ffi::OsStr::new("--experimental-strip-types"), file.as_os_str()];
    match output("node", &args, root) {
        Ok(output) if output.status.success() => Ok(format_output(output)),
        Ok(first) => {
            let detail = format_output(first);
            if detail.to_ascii_lowercase().contains("unknown file extension")
                || detail.to_ascii_lowercase().contains("strip-types")
                || detail.to_ascii_lowercase().contains("experimental-strip-types")
            {
                run_with_tsx(file, root)
            } else {
                Err(detail)
            }
        }
        Err(error) if (error.to_ascii_lowercase().contains("not found") || error.to_ascii_lowercase().contains("не найдена")) => {
            run_with_tsx(file, root)
        }
        Err(error) => Err(error),
    }
}

fn run_with_tsx(file: &Path, root: &Path) -> Result<String, String> {
    let entry = root.join("node_modules").join("tsx").join("dist").join("cli.mjs");
    if !entry.is_file() {
        return Err("Для TSX/JSX или Node без поддержки type stripping установи зависимости проекта: npm install".into());
    }
    run_program("node", &[entry.as_os_str(), file.as_os_str()], root)
}

/// Compile and run one selected Kotlin source file using the installed Kotlin CLI.
fn run_kotlin_source(file: &Path, root: &Path) -> Result<String, String> {
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
    let jar = std::env::temp_dir().join(format!("minux-kotlin-{}-{stamp}.jar", std::process::id()));
    let compile = Command::new("kotlinc")
        .arg(file)
        .arg("-include-runtime")
        .arg("-d")
        .arg(&jar)
        .current_dir(root)
        .output()
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                "Для запуска Kotlin установи JDK и Kotlin compiler (команда kotlinc) и добавь их в PATH.".to_string()
            } else {
                format!("Не удалось запустить Kotlin compiler: {error}")
            }
        })?;

    if !compile.status.success() {
        let detail = format_output(compile);
        let _ = fs::remove_file(&jar);
        return Err(format!("Ошибка компиляции Kotlin:\n{detail}"));
    }

    let result = output("java", &[std::ffi::OsStr::new("-jar"), jar.as_os_str()], root);
    let _ = fs::remove_file(&jar);
    result.and_then(|output| {
        if output.status.success() { Ok(format_output(output)) } else { Err(format_output(output)) }
    })
}

fn run_python(file: &Path, root: &Path) -> Result<String, String> {
    let args = [std::ffi::OsStr::new("-3"), file.as_os_str()];
    run_fallback("py", &args, "python", &[file.as_os_str()], root)
}

fn run_make(file: &Path, root: &Path) -> Result<String, String> {
    let working_directory = file.parent().unwrap_or(root);
    run_program("make", &[std::ffi::OsStr::new("-f"), file.as_os_str()], working_directory)
}

fn run_c(file: &Path, root: &Path) -> Result<String, String> {
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
    let mut binary = std::env::temp_dir().join(format!("minux-run-{}-{stamp}", std::process::id()));
    if cfg!(target_os = "windows") { binary.set_extension("exe"); }

    let compile_args = [
        std::ffi::OsStr::new("-std=c11"),
        std::ffi::OsStr::new("-O2"),
        file.as_os_str(),
        std::ffi::OsStr::new("-o"),
        binary.as_os_str(),
    ];
    let compile_output = output("cc", &compile_args, root)
        .map_err(|_| "Для запуска C установи GCC/Clang и добавь компилятор cc в PATH.".to_string())?;
    if !compile_output.status.success() {
        let detail = format_output(compile_output);
        return Err(format!("Ошибка компиляции C:\n{detail}"));
    }
    let execution = Command::new(&binary)
        .current_dir(root)
        .output()
        .map_err(|e| format!("Сборка C успешна, но файл не запустился: {e}"));
    let _ = fs::remove_file(&binary);
    execution.and_then(|result| {
        if result.status.success() { Ok(format_output(result)) } else { Err(format_output(result)) }
    })
}

fn run_xslt(file: &Path, root: &Path) -> Result<String, String> {
    let parent = file.parent().unwrap_or(root);
    let input = fs::read_dir(parent)
        .map_err(|e| format!("Не удалось найти XML для XSLT: {e}"))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("xml")))
        .min();
    let Some(input) = input else {
        return Err("Для запуска XSLT положи рядом с файлом входной .xml документ.".into());
    };
    run_program("xsltproc", &[file.as_os_str(), input.as_os_str()], root)
}

fn run_fallback(
    first: &str,
    first_args: &[&std::ffi::OsStr],
    second: &str,
    second_args: &[&std::ffi::OsStr],
    root: &Path,
) -> Result<String, String> {
    match output(first, first_args, root) {
        Ok(result) => {
            if result.status.success() { Ok(format_output(result)) } else { Err(format_output(result)) }
        }
        Err(error) if (error.to_ascii_lowercase().contains("not found") || error.to_ascii_lowercase().contains("не найдена")) => run_program(second, second_args, root),
        Err(error) => Err(error),
    }
}

fn run_program(program: &str, args: &[&std::ffi::OsStr], root: &Path) -> Result<String, String> {
    let result = output(program, args, root)?;
    if result.status.success() { Ok(format_output(result)) } else { Err(format_output(result)) }
}

fn output(program: &str, args: &[&std::ffi::OsStr], root: &Path) -> Result<Output, String> {
    Command::new(program)
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                format!("Команда '{program}' не найдена. Установи её и добавь в PATH.")
            } else {
                format!("Не удалось запустить '{program}': {error}")
            }
        })
}

fn format_output(result: Output) -> String {
    let mut text = String::new();
    if !result.stdout.is_empty() { text.push_str(&String::from_utf8_lossy(&result.stdout)); }
    if !result.stderr.is_empty() {
        if !text.is_empty() && !text.ends_with('\n') { text.push('\n'); }
        text.push_str(&String::from_utf8_lossy(&result.stderr));
    }
    if text.is_empty() { text.push_str("(команда завершилась без вывода)"); }
    if text.len() > MAX_OUTPUT_BYTES {
        let mut end = MAX_OUTPUT_BYTES;
        while !text.is_char_boundary(end) { end -= 1; }
        text.truncate(end);
        text.push_str("\n… вывод сокращён до 48 KiB");
    }
    text
}
