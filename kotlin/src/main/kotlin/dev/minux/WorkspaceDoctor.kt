package dev.minux

import java.io.IOException
import java.nio.charset.StandardCharsets
import java.nio.file.FileVisitOption
import java.nio.file.FileVisitResult
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import java.nio.file.SimpleFileVisitor
import java.nio.file.attribute.BasicFileAttributes
import java.util.EnumSet
import java.util.Locale
import java.util.TreeMap
import kotlin.system.exitProcess

data class TodoFinding(val file: String, val line: Int, val marker: String, val preview: String)

data class WorkspaceReport(
    val root: String,
    val totalFiles: Int,
    val languageCounts: Map<String, Int>,
    val buildFiles: List<String>,
    val todoFindings: List<TodoFinding>,
    val truncated: Boolean,
    val todoLimitReached: Boolean,
) {
    fun render(): String = buildString {
        appendLine("MINUX Workspace Doctor (Kotlin)")
        appendLine("Workspace: $root")
        appendLine("Files indexed: $totalFiles")
        appendLine("\nLanguages:")
        if (languageCounts.isEmpty()) appendLine("  (no recognized source files)")
        else languageCounts.forEach { (language, count) -> appendLine("  $language: $count") }
        appendLine("\nBuild manifests:")
        if (buildFiles.isEmpty()) appendLine("  (none detected)")
        else buildFiles.forEach { appendLine("  $it") }
        appendLine("\nTODO / FIXME / HACK / XXX:")
        if (todoFindings.isEmpty()) appendLine("  (none detected in bounded text scan)")
        else todoFindings.forEach { appendLine("  ${it.file}:${it.line} [${it.marker}] ${it.preview}") }
        if (todoLimitReached) appendLine("\nTODO scan stopped at the configured result limit.")
        if (truncated) appendLine("\nWorkspace scan stopped at its entry/depth safety limit.")
    }
}

/** Read-only project diagnostics. The scan is bounded and skips generated and secret-like paths. */
object WorkspaceDoctor {
    private const val DEFAULT_MAX_ENTRIES = 10_000
    private const val DEFAULT_MAX_DEPTH = 24
    private const val DEFAULT_MAX_TODOS = 100
    private const val MAX_TEXT_FILE_BYTES = 512L * 1024L

    private val ignoredDirectories = setOf(
        ".git", ".hg", ".svn", ".gradle", "target", "node_modules", "build", "dist",
        "out", "bin", "obj", "vendor", ".venv", "venv", "__pycache__", ".next", ".cache", "coverage",
    )
    private val languageByExtension = mapOf(
        "kt" to "Kotlin", "kts" to "Kotlin", "rs" to "Rust", "c" to "C",
        "h" to "C/C++ Header", "cc" to "C++", "cpp" to "C++", "cxx" to "C++", "c++" to "C++",
        "hpp" to "C++ Header", "hh" to "C++ Header", "hxx" to "C++ Header",
        "cs" to "C#", "fs" to "F#", "fsx" to "F#", "fsi" to "F#", "ts" to "TypeScript",
        "tsx" to "TypeScript", "mts" to "TypeScript", "cts" to "TypeScript",
        "js" to "JavaScript", "jsx" to "JavaScript", "mjs" to "JavaScript", "cjs" to "JavaScript",
        "coffee" to "CoffeeScript", "groovy" to "Groovy", "gradle" to "Gradle",
        "py" to "Python", "pyw" to "Python", "sh" to "Shell", "bash" to "Shell",
        "java" to "Java", "go" to "Go", "php" to "PHP", "rb" to "Ruby", "swift" to "Swift",
        "lua" to "Lua", "sql" to "SQL", "dart" to "Dart", "pl" to "Perl", "pm" to "Perl",
        "r" to "R", "scala" to "Scala", "hs" to "Haskell", "clj" to "Clojure",
        "cljs" to "Clojure", "erl" to "Erlang", "ex" to "Elixir", "exs" to "Elixir",
        "jl" to "Julia", "zig" to "Zig", "sol" to "Solidity", "tf" to "Terraform",
        "hcl" to "Terraform", "nix" to "Nix", "f" to "Fortran", "for" to "Fortran",
        "f90" to "Fortran", "f95" to "Fortran", "f03" to "Fortran", "f08" to "Fortran",
        "ml" to "OCaml", "mli" to "OCaml", "mm" to "Objective-C", "m" to "MATLAB",
        "vue" to "Vue", "svelte" to "Svelte", "html" to "HTML", "css" to "CSS",
        "scss" to "SCSS", "sass" to "Sass", "less" to "Less", "xml" to "XML", "xsl" to "XSLT",
        "md" to "Markdown", "mdx" to "MDX", "json" to "JSON", "jsonc" to "JSONC",
        "json5" to "JSON5", "jsonl" to "JSONL", "yaml" to "YAML", "yml" to "YAML",
        "toml" to "TOML", "ini" to "INI", "cfg" to "Config", "conf" to "Config",
        "ps1" to "PowerShell", "psm1" to "PowerShell", "psd1" to "PowerShell",
        "graphql" to "GraphQL", "gql" to "GraphQL", "tcl" to "Tcl",
        "tex" to "LaTeX", "ltx" to "LaTeX", "diff" to "Diff", "patch" to "Diff",
        "rmd" to "R Markdown", "rake" to "Ruby",
    )
    private val languageByFilename = mapOf(
        "dockerfile" to "Docker", "containerfile" to "Docker",
        "makefile" to "Make", "gnumakefile" to "Make", "justfile" to "Just",
        "cmakelists.txt" to "CMake", "jenkinsfile" to "Groovy",
    )
    private val textFileNames = setOf(
        "dockerfile", "containerfile", "makefile", "gnumakefile", "justfile",
        "cmakelists.txt", "jenkinsfile", ".gitignore", ".dockerignore",
    )
    private val manifestNames = setOf(
        "cargo.toml", "cargo.lock", "package.json", "package-lock.json", "tsconfig.json",
        "pyproject.toml", "requirements.txt", "requirements-dev.txt", "makefile", "gnumakefile",
        "cmakelists.txt", "go.mod", "pom.xml", "build.gradle", "build.gradle.kts",
        "settings.gradle", "settings.gradle.kts", "composer.json", "gemfile", "dockerfile",
        "containerfile", "justfile", "jenkinsfile", "gradlew", "gradlew.bat",
        "deno.json", "deno.jsonc", "bun.lockb", "mix.exs", "go.work", "cargo.config.toml",
    )
    private val textExtensions = setOf(
        "kt", "kts", "rs", "c", "h", "cc", "cpp", "cxx", "hpp", "hh", "hxx", "cs",
        "fs", "fsx", "fsi", "ts", "tsx", "mts", "cts", "js", "jsx", "mjs", "cjs", "coffee",
        "py", "pyw", "sh", "bash", "java", "go", "php", "rb", "rake", "swift", "groovy",
        "lua", "sql", "dart", "pl", "pm", "scala", "hs", "clj", "cljs", "cljc", "cljx",
        "erl", "hrl", "ex", "exs", "vue", "svelte", "html", "css", "scss", "sass", "less",
        "xml", "xsl", "xsd", "md", "mdx", "jl", "zig", "sol", "tf", "hcl", "nix",
        "f", "for", "f90", "f95", "f03", "f08", "ml", "mli", "mm", "m", "ps1", "psm1",
        "psd1", "graphql", "gql", "tcl", "tex", "ltx", "diff", "patch", "json5", "jsonl",
        "rmd", "ini", "cfg", "conf", "c++",
    )
    private val todoPattern = Regex("""(?i)^\s*(//|#|/\*|\*|<!--).*\b(TODO|FIXME|HACK|XXX)\b""")
    private val whitespacePattern = Regex("\\s+")

    fun inspect(
        path: Path,
        maxEntries: Int = DEFAULT_MAX_ENTRIES,
        maxDepth: Int = DEFAULT_MAX_DEPTH,
        maxTodoFindings: Int = DEFAULT_MAX_TODOS,
    ): WorkspaceReport {
        require(maxEntries > 0) { "maxEntries must be positive" }
        require(maxDepth > 0) { "maxDepth must be positive" }
        require(maxTodoFindings > 0) { "maxTodoFindings must be positive" }

        val root = path.toRealPath()
        require(Files.isDirectory(root)) { "Workspace is not a directory: $root" }
        var visitedEntries = 0
        var fileCount = 0
        var truncated = false
        var todoLimitReached = false
        val languages = TreeMap<String, Int>()
        val manifests = sortedSetOf<String>()
        val todos = mutableListOf<TodoFinding>()

        Files.walkFileTree(root, EnumSet.noneOf(FileVisitOption::class.java), maxDepth,
            object : SimpleFileVisitor<Path>() {
                override fun preVisitDirectory(dir: Path, attrs: BasicFileAttributes): FileVisitResult {
                    if (dir != root && dir.fileName.toString().lowercase(Locale.ROOT) in ignoredDirectories) {
                        return FileVisitResult.SKIP_SUBTREE
                    }
                    if (++visitedEntries > maxEntries) {
                        truncated = true
                        return FileVisitResult.TERMINATE
                    }
                    return FileVisitResult.CONTINUE
                }

                override fun visitFile(file: Path, attrs: BasicFileAttributes): FileVisitResult {
                    if (++visitedEntries > maxEntries) {
                        truncated = true
                        return FileVisitResult.TERMINATE
                    }
                    if (!attrs.isRegularFile || attrs.isSymbolicLink) return FileVisitResult.CONTINUE
                    fileCount++

                    val relative = root.relativize(file).toString().replace('\\', '/')
                    val normalizedName = file.fileName.toString().lowercase(Locale.ROOT)
                    val extension = normalizedName.substringAfterLast('.', "")
                    (languageByFilename[normalizedName] ?: languageByExtension[extension])?.let { language ->
                        languages[language] = (languages[language] ?: 0) + 1
                    }
                    if (normalizedName in manifestNames || normalizedName.endsWith(".csproj") ||
                        normalizedName.endsWith(".sln")) manifests += relative

                    if ((extension in textExtensions || normalizedName in textFileNames) &&
                        !isSecretPath(relative) && !todoLimitReached) {
                        val size = try { Files.size(file) } catch (_: IOException) { Long.MAX_VALUE }
                        if (size <= MAX_TEXT_FILE_BYTES) {
                            try {
                                Files.newBufferedReader(file, StandardCharsets.UTF_8).use { reader ->
                                    var lineNumber = 0
                                    while (true) {
                                        val line = reader.readLine() ?: break
                                        lineNumber++
                                        for (match in todoPattern.findAll(line)) {
                                            if (todos.size >= maxTodoFindings) {
                                                todoLimitReached = true
                                                break
                                            }
                                            val preview = line.trim().replace(whitespacePattern, " ").take(180)
                                            todos += TodoFinding(relative, lineNumber, match.groupValues.last().uppercase(Locale.ROOT), preview)
                                        }
                                        if (todoLimitReached) break
                                    }
                                }
                            } catch (_: IOException) {
                                // Unreadable and non-UTF-8 files are ignored.
                            }
                        }
                    }
                    return FileVisitResult.CONTINUE
                }

                override fun visitFileFailed(file: Path, error: IOException): FileVisitResult =
                    FileVisitResult.SKIP_SUBTREE
            },
        )

        return WorkspaceReport(
            root.toString(), fileCount, languages.toMap(), manifests.toList(), todos.toList(),
            truncated, todoLimitReached,
        )
    }

    private fun isSecretPath(relative: String): Boolean {
        val parts = relative.lowercase(Locale.ROOT).split('/')
        if (parts.any { it in setOf("secrets", ".secrets", "credentials") }) return true
        val name = parts.lastOrNull().orEmpty()
        return name.startsWith(".env") || name.contains("secret") || name.contains("credential") ||
            name.endsWith(".pem") || name.endsWith(".key") || name.endsWith(".p12") || name.endsWith(".pfx")
    }
}

fun main(args: Array<String>) {
    val root = args.firstOrNull()?.let { Paths.get(it) } ?: Paths.get(".")
    try {
        print(WorkspaceDoctor.inspect(root).render())
    } catch (error: Exception) {
        System.err.println("MINUX Workspace Doctor: ${error.message ?: error::class.simpleName}")
        exitProcess(2)
    }
}
