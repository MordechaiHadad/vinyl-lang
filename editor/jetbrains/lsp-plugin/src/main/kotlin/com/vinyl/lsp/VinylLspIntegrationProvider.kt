package com.vinyl.lsp

import com.intellij.openapi.project.Project
import com.intellij.openapi.vfs.VirtualFile
import com.intellij.platform.lsp.api.LspIntegrationProvider

/**
 * Starts a vinyl-lsp client when a .vn file is opened.
 *
 * The extension point is registered in META-INF/lsp.xml, which is only loaded
 * when the platform LSP module (`com.intellij.modules.lsp`) is present.
 */
class VinylLspIntegrationProvider : LspIntegrationProvider {

    override fun fileOpened(
        project: Project,
        file: VirtualFile,
        clientStarter: LspIntegrationProvider.LspClientStarter,
    ) {
        if (file.extension == "vn") {
            clientStarter.ensureClientStarted(VinylLspClientDescriptor(project))
        }
    }
}