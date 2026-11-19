package io.mediagallery.app

import android.Manifest
import android.content.ContentUris
import android.content.pm.PackageManager
import android.os.Build
import android.provider.MediaStore
import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel
import org.json.JSONArray
import org.json.JSONObject

class MainActivity : FlutterActivity() {
    companion object {
        private const val CHANNEL = "io.mediagallery.app/mediastore"
        private const val REQ_MEDIA_PERMS = 9101
    }

    private var pendingPermResult: MethodChannel.Result? = null

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)
        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, CHANNEL).setMethodCallHandler { call, result ->
            when (call.method) {
                "getAppDirs" -> {
                    result.success(
                        mapOf(
                            "filesDir" to filesDir.absolutePath,
                            "cacheDir" to cacheDir.absolutePath,
                            "deviceName" to "${Build.MANUFACTURER} ${Build.MODEL}".trim()
                        )
                    )
                }
                "scanMediaStore" -> {
                    if (!hasMediaPermission()) {
                        pendingPermResult = result
                        requestMediaPermissions()
                    } else {
                        Thread {
                            try {
                                val json = queryMediaStoreJson()
                                runOnUiThread { result.success(json) }
                            } catch (e: Exception) {
                                runOnUiThread { result.error("MEDIASTORE_ERR", e.message, null) }
                            }
                        }.start()
                    }
                }
                else -> result.notImplemented()
            }
        }
    }

    private fun requiredPermissions(): Array<String> {
        return if (Build.VERSION.SDK_INT >= 33) {
            arrayOf(
                Manifest.permission.READ_MEDIA_IMAGES,
                Manifest.permission.READ_MEDIA_VIDEO,
                Manifest.permission.READ_MEDIA_AUDIO
            )
        } else {
            arrayOf(Manifest.permission.READ_EXTERNAL_STORAGE)
        }
    }

    private fun hasMediaPermission(): Boolean {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.M) return true
        return requiredPermissions().any {
            checkSelfPermission(it) == PackageManager.PERMISSION_GRANTED
        }
    }

    private fun requestMediaPermissions() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
            requestPermissions(requiredPermissions(), REQ_MEDIA_PERMS)
        }
    }

    override fun onRequestPermissionsResult(
        requestCode: Int,
        permissions: Array<out String>,
        grantResults: IntArray
    ) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults)
        if (requestCode == REQ_MEDIA_PERMS) {
            val res = pendingPermResult
            pendingPermResult = null
            if (res != null) {
                if (hasMediaPermission()) {
                    Thread {
                        try {
                            val json = queryMediaStoreJson()
                            runOnUiThread { res.success(json) }
                        } catch (e: Exception) {
                            runOnUiThread { res.error("MEDIASTORE_ERR", e.message, null) }
                        }
                    }.start()
                } else {
                    res.success("[]")
                }
            }
        }
    }

    private fun queryMediaStoreJson(): String {
        val arr = JSONArray()
        val collection = MediaStore.Files.getContentUri("external")
        val cols = mutableListOf(
            MediaStore.Files.FileColumns._ID,
            MediaStore.Files.FileColumns.DISPLAY_NAME,
            MediaStore.Files.FileColumns.MIME_TYPE,
            MediaStore.Files.FileColumns.SIZE,
            MediaStore.Files.FileColumns.DATE_ADDED,
            MediaStore.Files.FileColumns.DATE_MODIFIED,
            MediaStore.MediaColumns.DURATION,
            MediaStore.MediaColumns.WIDTH,
            MediaStore.MediaColumns.HEIGHT
        )
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            cols.add(MediaStore.Files.FileColumns.RELATIVE_PATH)
        }

        val selection = "${MediaStore.Files.FileColumns.MEDIA_TYPE} IN (?, ?, ?)"
        val args = arrayOf(
            MediaStore.Files.FileColumns.MEDIA_TYPE_IMAGE.toString(),
            MediaStore.Files.FileColumns.MEDIA_TYPE_VIDEO.toString(),
            MediaStore.Files.FileColumns.MEDIA_TYPE_AUDIO.toString()
        )

        contentResolver.query(
            collection,
            cols.toTypedArray(),
            selection,
            args,
            "${MediaStore.Files.FileColumns.DATE_MODIFIED} DESC"
        )?.use { cursor ->
            val idIdx = cursor.getColumnIndex(MediaStore.Files.FileColumns._ID)
            val nameIdx = cursor.getColumnIndex(MediaStore.Files.FileColumns.DISPLAY_NAME)
            val mimeIdx = cursor.getColumnIndex(MediaStore.Files.FileColumns.MIME_TYPE)
            val sizeIdx = cursor.getColumnIndex(MediaStore.Files.FileColumns.SIZE)
            val addedIdx = cursor.getColumnIndex(MediaStore.Files.FileColumns.DATE_ADDED)
            val modIdx = cursor.getColumnIndex(MediaStore.Files.FileColumns.DATE_MODIFIED)
            val durIdx = cursor.getColumnIndex(MediaStore.MediaColumns.DURATION)
            val wIdx = cursor.getColumnIndex(MediaStore.MediaColumns.WIDTH)
            val hIdx = cursor.getColumnIndex(MediaStore.MediaColumns.HEIGHT)
            val relIdx = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                cursor.getColumnIndex(MediaStore.Files.FileColumns.RELATIVE_PATH)
            } else {
                -1
            }

            while (cursor.moveToNext()) {
                if (idIdx < 0) continue
                val id = cursor.getLong(idIdx)
                val uri = ContentUris.withAppendedId(collection, id).toString()
                val obj = JSONObject()
                obj.put("uri", uri)
                obj.put("display_name", if (nameIdx >= 0) (cursor.getString(nameIdx) ?: "unknown") else "unknown")
                obj.put("relative_path", if (relIdx >= 0) (cursor.getString(relIdx) ?: "") else "")
                obj.put("mime_type", if (mimeIdx >= 0) (cursor.getString(mimeIdx) ?: "application/octet-stream") else "application/octet-stream")
                obj.put("size", if (sizeIdx >= 0) cursor.getLong(sizeIdx).coerceAtLeast(0L) else 0L)
                obj.put("date_added_sec", if (addedIdx >= 0) cursor.getLong(addedIdx) else 0L)
                obj.put("date_modified_sec", if (modIdx >= 0) cursor.getLong(modIdx) else 0L)
                obj.put("duration_ms", if (durIdx >= 0) cursor.getLong(durIdx).coerceAtLeast(0L) else 0L)
                obj.put("width", if (wIdx >= 0) cursor.getInt(wIdx).coerceAtLeast(0) else 0)
                obj.put("height", if (hIdx >= 0) cursor.getInt(hIdx).coerceAtLeast(0) else 0)
                arr.put(obj)
            }
        }
        return arr.toString()
    }
}
