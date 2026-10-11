package dev.minux

import java.nio.file.Files
import java.nio.file.Path
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue
import org.junit.jupiter.api.io.TempDir

class WorkspaceDoctorTest {
    @TempDir
    lateinit var workspace: Path

    @Test
    fun reportsLanguagesManifestsAndTodoMarkers() {
        Files.createDirectories(workspace.resolve("src"))
        Files.writeString(workspace.resolve("Cargo.toml"), "[package]\nname = \"demo\"\n")
        Files.writeString(workspace.resolve("src/App.kt"), "fun main() {}\n// TODO: add validation\n")
        Files.writeString(workspace.resolve("src/lib.rs"), "fn main() {}\n")
        Files.writeString(workspace.resolve("web.ts"), "const value: number = 1;\n")
        Files.writeString(workspace.resolve("science.jl"), "module Science\n")
        Files.writeString(workspace.resolve("native.zig"), "pub fn main() void {}\n")
        Files.createDirectories(workspace.resolve("native"))
        Files.createDirectories(workspace.resolve("infra"))
        Files.createDirectories(workspace.resolve("web"))
        Files.createDirectories(workspace.resolve("styles"))
        Files.createDirectories(workspace.resolve("analysis"))
        Files.createDirectories(workspace.resolve("scripts"))
        Files.createDirectories(workspace.resolve("docs"))
        Files.createDirectories(workspace.resolve("build"))
        Files.writeString(workspace.resolve("native/engine.cpp"), "int main() { return 0; }\n")
        Files.writeString(workspace.resolve("script.fsx"), "printfn \"ok\"\n")
        Files.writeString(workspace.resolve("infra/Containerfile"), "FROM scratch\n")
        Files.writeString(workspace.resolve("web/schema.graphql"), "type Query { health: String }\n")
        Files.writeString(workspace.resolve("styles/theme.scss"), "$accent: blue;\n")
        Files.writeString(workspace.resolve("analysis/report.Rmd"), "---\ntitle: demo\n---\n")
        Files.writeString(workspace.resolve("scripts/deploy.ps1"), "Write-Output 'ok'\n")
        Files.writeString(workspace.resolve("docs/readme.tex"), "\\documentclass{article}\n")
        Files.writeString(workspace.resolve("scripts/Makefile"), "all:\n\t@echo ok\n")

        val report = WorkspaceDoctor.inspect(workspace)

        assertEquals(1, report.languageCounts["Kotlin"])
        assertEquals(1, report.languageCounts["Rust"])
        assertEquals(1, report.languageCounts["TypeScript"])
        assertEquals(1, report.languageCounts["Julia"])
        assertEquals(1, report.languageCounts["Zig"])
        assertEquals(1, report.languageCounts["C++"])
        assertEquals(1, report.languageCounts["F#"])
        assertEquals(1, report.languageCounts["Docker"])
        assertEquals(1, report.languageCounts["GraphQL"])
        assertEquals(1, report.languageCounts["SCSS"])
        assertEquals(1, report.languageCounts["R Markdown"])
        assertEquals(1, report.languageCounts["PowerShell"])
        assertEquals(1, report.languageCounts["LaTeX"])
        assertEquals(1, report.languageCounts["Make"])
        assertTrue(report.buildFiles.contains("Cargo.toml"))
        assertTrue(report.buildFiles.contains("scripts/Makefile"))
        assertEquals(1, report.todoFindings.size)
        assertEquals("TODO", report.todoFindings.single().marker)
        assertEquals(2, report.todoFindings.single().line)
        assertEquals("TODO", report.todoFindings.last().marker)
    }

    @Test
    fun skipsGeneratedDirectoriesAndSecretPaths() {
        Files.createDirectories(workspace.resolve("target/generated"))
        Files.createDirectories(workspace.resolve("src"))
        Files.writeString(workspace.resolve("target/generated/Generated.kt"), "// FIXME: not source\n")
        Files.writeString(workspace.resolve("src/real.kt"), "// TODO: inspect this\n")
        Files.writeString(workspace.resolve("src/.env.kt"), "// TODO secret token must be skipped\n")

        val report = WorkspaceDoctor.inspect(workspace)

        assertFalse(report.buildFiles.any { it.startsWith("target/") })
        assertEquals(1, report.todoFindings.size)
        assertEquals("src/real.kt", report.todoFindings.single().file)
    }

    @Test
    fun stopsAtEntryLimit() {
        Files.createDirectories(workspace.resolve("src"))
        repeat(8) { index -> Files.writeString(workspace.resolve("src/File$index.kt"), "class File$index\n") }
        val report = WorkspaceDoctor.inspect(workspace, maxEntries = 3)
        assertTrue(report.truncated)
        assertTrue(report.totalFiles < 8)
    }

    @Test
    fun rejectsInvalidLimits() {
        kotlin.test.assertFailsWith<IllegalArgumentException> {
            WorkspaceDoctor.inspect(workspace, maxEntries = 0)
        }
    }
}
