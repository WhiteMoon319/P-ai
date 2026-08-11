package com.whitemoon319.pai.model

import com.google.gson.JsonElement
import com.google.gson.annotations.SerializedName

// ---------------- Android 工作区文件管理 ----------------

data class WorkspaceFileEntry(
    val name: String? = null,
    val path: String? = null,
    val kind: String? = null,
    val bytes: Long? = null,
) {
    val isDirectory: Boolean get() = kind == "directory"
}

data class WorkspaceFileListResult(
    @SerializedName("currentPath") val currentPath: String? = null,
    @SerializedName("parentPath") val parentPath: String? = null,
    val entries: List<WorkspaceFileEntry> = emptyList(),
)

data class WorkspaceTextResult(
    val path: String? = null,
    val text: String? = null,
    val bytes: Long = 0,
)

data class WorkspaceWriteResult(
    val entry: WorkspaceFileEntry? = null,
)

data class WorkspaceMoveResult(
    @SerializedName("sourcePath") val sourcePath: String? = null,
    val entry: WorkspaceFileEntry? = null,
)

data class WorkspaceGlobResult(
    val entries: List<WorkspaceFileEntry> = emptyList(),
)

data class WorkspaceSearchMatch(
    val path: String? = null,
    val line: Long = 0,
    val text: String? = null,
)

data class WorkspaceGrepResult(
    val matches: List<WorkspaceSearchMatch> = emptyList(),
)

data class WorkspaceDeleteResult(
    @SerializedName("deletedPath") val deletedPath: String? = null,
)

data class WorkspaceImportResult(
    val status: AndroidWorkspaceStatus? = null,
    @SerializedName("importedPath") val importedPath: String? = null,
    @SerializedName("fileName") val fileName: String? = null,
    val bytes: Long = 0,
)

data class WorkspaceExportResult(
    val path: String? = null,
    @SerializedName("fileName") val fileName: String? = null,
    val mime: String? = null,
    @SerializedName("dataBase64") val dataBase64: String? = null,
    val bytes: Long = 0,
)
