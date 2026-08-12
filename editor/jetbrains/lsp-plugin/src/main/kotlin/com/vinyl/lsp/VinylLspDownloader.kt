package com.vinyl.lsp

import com.intellij.openapi.application.PathManager
import com.intellij.openapi.util.SystemInfo
import com.google.gson.JsonObject
import com.google.gson.JsonParser
import org.apache.commons.compress.archivers.ArchiveEntry
import org.apache.commons.compress.archivers.ArchiveInputStream
import org.apache.commons.compress.archivers.tar.TarArchiveInputStream
import org.apache.commons.compress.archivers.zip.ZipArchiveInputStream
import org.apache.commons.compress.compressors.gzip.GzipCompressorInputStream
import java.io.InputStream
import java.net.HttpURLConnection
import java.net.URI
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.StandardCopyOption
import java.nio.file.attribute.PosixFilePermission

/**
 * Resolves the vinyl-lsp binary: explicit path from the `vinyl.lsp.path`
 * system property, then `$PATH`, then a download from the latest GitHub
 * release into the IDE system directory (cached per release tag).
 */
object VinylLspDownloader {

    private const val GITHUB_REPO = "MordechaiHadad/vinyl-lang"
    private const val SERVER_NAME = "vinyl-lsp"
    private const val API_LATEST_RELEASE =
        "https://api.github.com/repos/$GITHUB_REPO/releases/latest"

    @Volatile
    private var cachedPath: String? = null

    @Synchronized
    fun binaryPath(): String {
        cachedPath?.let { return it }

        System.getProperty("vinyl.lsp.path")?.let { explicit ->
            if (Files.isRegularFile(Path.of(explicit))) return remember(explicit)
        }

        findOnPath()?.let { return remember(it) }

        return remember(download())
    }

    private fun remember(path: String): String {
        cachedPath = path
        return path
    }

    private fun binaryName(): String =
        if (SystemInfo.isWindows) "$SERVER_NAME.exe" else SERVER_NAME

    private fun findOnPath(): String? {
        val pathEnv = System.getenv("PATH") ?: return null
        val binary = binaryName()
        return pathEnv.split(System.getProperty("path.separator"))
            .map { Path.of(it, binary) }
            .firstOrNull { Files.isRegularFile(it) }
            ?.toString()
    }

    private fun download(): String {
        val base = Path.of(PathManager.getSystemPath(), SERVER_NAME)
        val release = fetchLatestRelease()
        val tag = release["tag_name"].asString

        val versioned = base.resolve(tag)
        val versionedBinary = versioned.resolve(binaryName())
        if (Files.isRegularFile(versionedBinary)) return versionedBinary.toString()

        val (assetName, extension) = assetName()
        val asset = release["assets"].asJsonArray
            .map { it.asJsonObject }
            .firstOrNull { it["name"].asString == assetName }
            ?: error("Could not find asset $assetName in $GITHUB_REPO release $tag")

        Files.createDirectories(versioned)
        val archive = versioned.resolve(assetName)
        downloadTo(asset["browser_download_url"].asString, archive)
        try {
            extract(archive, versioned, extension)
        } finally {
            Files.deleteIfExists(archive)
        }
        if (SystemInfo.isUnix) {
            Files.setPosixFilePermissions(
                versionedBinary,
                setOf(
                    PosixFilePermission.OWNER_READ,
                    PosixFilePermission.OWNER_WRITE,
                    PosixFilePermission.OWNER_EXECUTE,
                    PosixFilePermission.GROUP_READ,
                    PosixFilePermission.GROUP_EXECUTE,
                    PosixFilePermission.OTHERS_READ,
                    PosixFilePermission.OTHERS_EXECUTE,
                ),
            )
        }
        return versionedBinary.toString()
    }

    private fun assetName(): Pair<String, String> {
        val architecture = System.getProperty("os.arch").lowercase()
        val arm = architecture == "aarch64" || architecture == "arm64"
        val (triple, extension) = when {
            SystemInfo.isMac && arm -> "aarch64-apple-darwin" to "tar.gz"
            SystemInfo.isMac -> "x86_64-apple-darwin" to "tar.gz"
            SystemInfo.isLinux && arm -> "aarch64-unknown-linux-gnu" to "tar.gz"
            SystemInfo.isLinux -> "x86_64-unknown-linux-gnu" to "tar.gz"
            SystemInfo.isWindows && arm -> "aarch64-pc-windows-msvc" to "zip"
            else -> "x86_64-pc-windows-msvc" to "zip"
        }
        return "vinyl-lsp-$triple.$extension" to extension
    }

    private fun fetchLatestRelease(): JsonObject {
        val connection = URI(API_LATEST_RELEASE).toURL().openConnection() as HttpURLConnection
        connection.requestMethod = "GET"
        connection.setRequestProperty("Accept", "application/vnd.github+json")
        connection.setRequestProperty("User-Agent", "vinyl-jetbrains-plugin")
        return try {
            val body = connection.inputStream.bufferedReader().use { it.readText() }
            JsonParser.parseString(body).asJsonObject
        } finally {
            connection.disconnect()
        }
    }

    private fun downloadTo(url: String, target: Path) {
        val connection = URI(url).toURL().openConnection() as HttpURLConnection
        connection.setRequestProperty("User-Agent", "vinyl-jetbrains-plugin")
        try {
            connection.inputStream.use { input ->
                Files.copy(input, target, StandardCopyOption.REPLACE_EXISTING)
            }
        } finally {
            connection.disconnect()
        }
    }

    private fun extract(archive: Path, destination: Path, extension: String) {
        val raw = Files.newInputStream(archive)
        if (extension == "zip") {
            ZipArchiveInputStream(raw).use { extractEntries(it, destination) }
        } else {
            TarArchiveInputStream(GzipCompressorInputStream(raw)).use {
                extractEntries(it, destination)
            }
        }
    }

    private fun extractEntries(
        stream: ArchiveInputStream<out ArchiveEntry>,
        destination: Path,
    ) {
        while (true) {
            val entry: ArchiveEntry = stream.getNextEntry() ?: break
            if (entry.isDirectory || entry.name.isBlank()) continue
            val target = destination.resolve(entry.name).normalize()
            check(target.startsWith(destination)) { "archive entry escapes destination: ${entry.name}" }
            Files.createDirectories(target.parent)
            Files.newOutputStream(target).use { out -> stream.copyTo(out) }
        }
    }
}
