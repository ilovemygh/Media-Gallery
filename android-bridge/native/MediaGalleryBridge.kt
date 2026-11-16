package io.mediagallery.app

import android.content.ContentUris
import android.content.Context
import android.graphics.Bitmap
import android.media.MediaMetadataRetriever
import android.net.Uri
import android.provider.MediaStore
import java.io.File
import java.io.FileOutputStream

/**
 * Native Android Kotlin bridge supporting:
 * - MediaStore incremental scanning (scoped storage compatible)
 * - MediaMetadataRetriever frame extraction at exact frameTimeMs (PRD Section 4 & I1)
 */
class MediaGalleryBridge(private val context: Context) {

    data class ScannedMediaRow(
        val uri: String,
        val displayName: String,
        val relativePath: String,
        val mimeType: String,
        val size: Long,
        val dateAddedSec: Long,
        val dateModifiedSec: Long,
        val durationMs: Long,
        val width: Int,
        val height: Int
    )

    fun queryMediaStore(sinceModifiedSec: Long = 0L): List<ScannedMediaRow> {
        val results = mutableListOf<ScannedMediaRow>()
        val collection = MediaStore.Files.getContentUri("external")
        val projection = arrayOf(
            MediaStore.Files.FileColumns._ID,
            MediaStore.Files.FileColumns.DISPLAY_NAME,
            MediaStore.Files.FileColumns.RELATIVE_PATH,
            MediaStore.Files.FileColumns.MIME_TYPE,
            MediaStore.Files.FileColumns.SIZE,
            MediaStore.Files.FileColumns.DATE_ADDED,
            MediaStore.Files.FileColumns.DATE_MODIFIED,
            MediaStore.Files.FileColumns.MEDIA_TYPE,
            MediaStore.MediaColumns.DURATION,
            MediaStore.MediaColumns.WIDTH,
            MediaStore.MediaColumns.HEIGHT
        )
        val selection = "${MediaStore.Files.FileColumns.MEDIA_TYPE} IN (?, ?, ?) AND ${MediaStore.Files.FileColumns.DATE_MODIFIED} >= ?"
        val args = arrayOf(
            MediaStore.Files.FileColumns.MEDIA_TYPE_IMAGE.toString(),
            MediaStore.Files.FileColumns.MEDIA_TYPE_VIDEO.toString(),
            MediaStore.Files.FileColumns.MEDIA_TYPE_AUDIO.toString(),
            sinceModifiedSec.toString()
        )

        context.contentResolver.query(collection, projection, selection, args, "${MediaStore.Files.FileColumns.DATE_MODIFIED} DESC")?.use { cursor ->
            val idCol = cursor.getColumnIndexOrThrow(MediaStore.Files.FileColumns._ID)
            val nameCol = cursor.getColumnIndexOrThrow(MediaStore.Files.FileColumns.DISPLAY_NAME)
            val pathCol = cursor.getColumnIndexOrThrow(MediaStore.Files.FileColumns.RELATIVE_PATH)
            val mimeCol = cursor.getColumnIndexOrThrow(MediaStore.Files.FileColumns.MIME_TYPE)
            val sizeCol = cursor.getColumnIndexOrThrow(MediaStore.Files.FileColumns.SIZE)
            val addedCol = cursor.getColumnIndexOrThrow(MediaStore.Files.FileColumns.DATE_ADDED)
            val modCol = cursor.getColumnIndexOrThrow(MediaStore.Files.FileColumns.DATE_MODIFIED)
            val durCol = cursor.getColumnIndexOrThrow(MediaStore.MediaColumns.DURATION)
            val wCol = cursor.getColumnIndexOrThrow(MediaStore.MediaColumns.WIDTH)
            val hCol = cursor.getColumnIndexOrThrow(MediaStore.MediaColumns.HEIGHT)

            while (cursor.moveToNext()) {
                val id = cursor.getLong(idCol)
                val uri = ContentUris.withAppendedId(collection, id).toString()
                results.add(
                    ScannedMediaRow(
                        uri = uri,
                        displayName = cursor.getString(nameCol) ?: "unknown",
                        relativePath = cursor.getString(pathCol) ?: "",
                        mimeType = cursor.getString(mimeCol) ?: "application/octet-stream",
                        size = cursor.getLong(sizeCol),
                        dateAddedSec = cursor.getLong(addedCol),
                        dateModifiedSec = cursor.getLong(modCol),
                        durationMs = cursor.getLong(durCol),
                        width = cursor.getInt(wCol),
                        height = cursor.getInt(hCol)
                    )
                )
            }
        }
        return results
    }

    fun extractVideoFrameAtMs(contentUri: String, frameTimeMs: Long, outputFile: File): String {
        val retriever = MediaMetadataRetriever()
        return try {
            retriever.setDataSource(context, Uri.parse(contentUri))
            val bitmap = retriever.getFrameAtTime(frameTimeMs * 1000L, MediaMetadataRetriever.OPTION_CLOSEST_SYNC)
                ?: throw IllegalStateException("Failed to decode frame for $contentUri")
            outputFile.parentFile?.mkdirs()
            FileOutputStream(outputFile).use { out ->
                bitmap.compress(Bitmap.CompressFormat.JPEG, 85, out)
            }
            bitmap.recycle()
            outputFile.absolutePath
        } finally {
            retriever.release()
        }
    }
}
