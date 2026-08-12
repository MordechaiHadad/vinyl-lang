package com.vinyl.lsp

import com.intellij.execution.configurations.GeneralCommandLine
import com.intellij.openapi.project.Project
import com.intellij.openapi.vfs.VirtualFile
import com.intellij.platform.lsp.api.ProjectWideLspClientDescriptor

/**
 * A single vinyl-lsp process serving the whole project.
 */
class VinylLspClientDescriptor(project: Project) :
    ProjectWideLspClientDescriptor(project, "Vinyl LSP") {

    override fun isSupportedFile(file: VirtualFile): Boolean = file.extension == "vn"

    override fun createCommandLine(): GeneralCommandLine =
        GeneralCommandLine(VinylLspDownloader.binaryPath()).withCharset(Charsets.UTF_8)
}