package io.mediagallery.plugins.sharesheet

import android.app.Activity
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.os.Bundle

/**
 * Android Share-sheet target ("Send with Media Gallery", PRD Sec 8 & 10.8).
 * Accepts ACTION_SEND and ACTION_SEND_MULTIPLE media URIs and forwards them
 * to the Flutter/Rust transfer screen.
 */
class ShareTargetActivity : Activity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val uris = extractSharedUris(intent)
        // Forward extracted URIs to MainActivity / Flutter MethodChannel ("share/incoming")
        val launchIntent = packageManager.getLaunchIntentForPackage(packageName)?.apply {
            putStringArrayListExtra("shared_media_uris", ArrayList(uris.map { it.toString() }))
            addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_SINGLE_TOP)
        }
        if (launchIntent != null) {
            startActivity(launchIntent)
        }
        finish()
    }

    private fun extractSharedUris(intent: Intent?): List<Uri> {
        if (intent == null) return emptyList()
        return when (intent.action) {
            Intent.ACTION_SEND -> {
                val single = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
                    intent.getParcelableExtra(Intent.EXTRA_STREAM, Uri::class.java)
                } else {
                    @Suppress("DEPRECATION")
                    intent.getParcelableExtra(Intent.EXTRA_STREAM)
                }
                listOfNotNull(single)
            }
            Intent.ACTION_SEND_MULTIPLE -> {
                val many = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
                    intent.getParcelableArrayListExtra(Intent.EXTRA_STREAM, Uri::class.java)
                } else {
                    @Suppress("DEPRECATION")
                    intent.getParcelableArrayListExtra(Intent.EXTRA_STREAM)
                }
                many.orEmpty()
            }
            else -> emptyList()
        }
    }
}
