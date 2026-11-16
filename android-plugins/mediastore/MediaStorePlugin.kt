package io.mediagallery.plugins.mediastore

import android.content.ContentResolver
import android.content.ContentUris
import android.content.Context
import android.database.ContentObserver
import android.graphics.Bitmap
import android.media.MediaMetadataRetriever
import android.net.Uri
import android.os.Handler
import android.os.Looper
import android.provider.MediaStore
import java.io.File
import java.io.FileOutputStream

/**
 * Kotlin MediaStore plugin implementing PRD Section 8:
 * - Queries MediaStore.Files with scoped storage compatibility (avoids MANAGE_EXTERNAL_STORAGE).
 * - Watches MediaStore changes via ContentObserver and streams batches to Rust `ingest_android_mediastore_batch`.
 * - Extracts exact frame thumbnails using `MediaMetadataRetriever` at `frameTimeMs`.
 */
class MediaStorePlugin(
    private val context: Context,
    private val onBatchReady: (List<Map<String, Any?>>) -> Unit
) {
    private var observer: ContentObserver? = null

    fun scanSince(sinceModifiedSec: Long = 0L, batchSize: Int = 250): List<Map<String, Any?>> {
        val rows = mutableListOf<Map<String, Any?>>()
        val collection = MediaStore.Files.getContentUri("external")
        val projection = arrayOf(
            MediaStore.Files.FileColumns._ID,
            MediaStore.Files.FileColumns.DISPLAY_NAME,
            MediaStore.Files.FileColumns.RELATIVE_PATH,
            MediaStore.Files.FileColumns.MIME_TYPE,
            MediaStore.Files.FileColumns.SIZE,
            MediaStore.Files.FileColumns.DATE_ADDED,
            MediaStore.Files.FileColumns.DATE_MODIFIED,
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

        context.contentResolver.query(
            collection,
            projection,
            selection,
            args,
            "${MediaStore.Files.FileColumns.DATE_MODIFIED} DESC"
        )?.use { cursor ->
            val idCol = cursor.getColumnIndexOrThrow(MediaStore.Files.FileColumns._ID)
            val nameCol = cursor.getColumnIndexOrThrow(MediaStore.Files.FileColumns.DISPLAY_NAME)
            val relCol = cursor.getColumnIndexOrThrow(MediaStore.Files.FileColumns.RELATIVE_PATH)
            val mimeCol = cursor.getColumnIndexOrThrow(MediaStore.Files.FileColumns.MIME_TYPE)
            val sizeCol = cursor.getColumnIndexOrThrow(MediaStore.Files.FileColumns.SIZE)
            val addedCol = cursor.getColumnIndexOrThrow(MediaStore.Files.FileColumns.DATE_ADDED)
            val modCol = cursor.getColumnIndexOrThrow(MediaStore.Files.FileColumns.DATE_MODIFIED)
            val durCol = cursor.getColumnIndexOrThrow(MediaStore.MediaColumns.DURATION)
            val wCol = cursor.getColumnIndexOrThrow(MediaStore.MediaColumns.WIDTH)
            val hCol = cursor.getColumnIndexOrThrow(MediaStore.MediaColumns.HEIGHT)

            val batch = mutableListOf<Map<String, Any?>>()
            while (cursor.moveToNext()) {
                val id = cursor.getLong(idCol)
                val uri = ContentUris.withAppendedId(collection, id).toString()
                val row = mapOf(
                    "uri" to uri,
                    "display_name" to (cursor.getString(nameCol) ?: "unknown"),
                    "relative_path" to (cursor.getString(relCol) ?: ""),
                    "mime_type" to (cursor.getString(mimeCol) ?: "application/octet-stream"),
                    "size" to cursor.getLong(sizeCol),
                    "date_added_sec" to cursor.getLong(addedCol),
                    "date_modified_sec" to cursor.getLong(modCol),
                    "duration_ms" to cursor.getLong(durCol),
                    "width" to cursor.getInt(wCol),
                    "height" to cursor.getInt(hCol)
                )
                rows.add(row)
                batch.add(row)
                if (batch.size >= batchSize) {
                    onBatchReady(batch.toList())
                    batch.clear()
                }
            }
            if (batch.isNotEmpty()) {
                onBatchReady(batch.toList())
            }
        }
        return rows
    }

    fun startObserver(lastKnownSecProvider: () -> Long) {
        stopObserver()
        val resolver: ContentResolver = context.contentResolver
        val newObserver = object : ContentObserver(Handler(Looper.getMainLooper())) {
            override fun onChange(selfChange: Boolean) {
                scanSince(lastKnownSecProvider())
            }
        }
        resolver.registerContentObserver(MediaStore.Images.Media.EXTERNAL_CONTENT_URI, true, newObserver)
        resolver.registerContentObserver(MediaStore.Video.Media.EXTERNAL_CONTENT_URI, true, newObserver)
        resolver.registerContentObserver(MediaStore.Audio.Media.EXTERNAL_CONTENT_URI, true, newObserver)
        observer = newObserver
    }

    fun stopObserver() {
        observer?.let { context.contentResolver.unregisterContentObserver(it) }
        observer = null
    }

    fun extractVideoFrame(contentUri: String, frameTimeMs: Long, outputFile: File): String {
        val retriever = MediaMetadataRetriever()
        return try {
            retriever.setDataSource(context, Uri.parse(contentUri))
            val bmp = retriever.getFrameAtTime(frameTimeMs * 1000L, MediaMetadataRetriever.OPTION_CLOSEST_SYNC)
                ?: throw IllegalStateException("MediaMetadataRetriever failed for $contentUri")
            outputFile.parentFile?.mkdirs()
            FileOutputStream(outputFile).use { out ->
                bmp.compress(Bitmap.CompressFormat.JPEG, 85, out)
            }
            bmp.recycle()
            outputFile.absolutePath
        } finally {
            retriever.release()
        }
    }
}
