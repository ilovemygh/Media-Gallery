package io.mediagallery.plugins.fgservice

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.Service
import android.content.Intent
import android.os.Build
import android.os.IBinder

/**
 * Foreground service started ONLY while transfers, downloads, or scans are active (PRD Sec 8 & 15).
 */
class ForegroundTransferService : Service() {
    companion object {
        const val CHANNEL_ID = "media_gallery_active_work"
        const val EXTRA_TITLE = "title"
        const val EXTRA_BODY = "body"
        private const val NOTIFICATION_ID = 4102
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        ensureChannel()
        val title = intent?.getStringExtra(EXTRA_TITLE) ?: "Media Gallery active"
        val body = intent?.getStringExtra(EXTRA_BODY) ?: "Transferring or syncing library..."
        val notification = buildNotification(title, body)
        startForeground(NOTIFICATION_ID, notification)
        return START_NOT_STICKY
    }

    private fun ensureChannel() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val mgr = getSystemService(NotificationManager::class.java)
            val channel = NotificationChannel(
                CHANNEL_ID,
                "Media Gallery Transfers & Sync",
                NotificationManager.IMPORTANCE_LOW
            )
            mgr.createNotificationChannel(channel)
        }
    }

    private fun buildNotification(title: String, body: String): Notification {
        val builder = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            Notification.Builder(this, CHANNEL_ID)
        } else {
            @Suppress("DEPRECATION")
            Notification.Builder(this)
        }
        return builder
            .setContentTitle(title)
            .setContentText(body)
            .setSmallIcon(android.R.drawable.stat_sys_download)
            .setOngoing(true)
            .build()
    }

    override fun onBind(intent: Intent?): IBinder? = null
}
