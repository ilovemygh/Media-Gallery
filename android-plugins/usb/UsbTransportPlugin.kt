package io.mediagallery.plugins.usb

import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.os.BatteryManager
import java.net.NetworkInterface

/**
 * Detects USB connection / USB tethering IP path for wired iroh/QUIC transfers (PRD Sec 8 & D4).
 */
class UsbTransportPlugin(private val context: Context) {
    fun isUsbConnected(): Boolean {
        val intent = context.registerReceiver(null, IntentFilter(Intent.ACTION_BATTERY_CHANGED))
        val plugged = intent?.getIntExtra(BatteryManager.EXTRA_PLUGGED, -1) ?: -1
        return plugged == BatteryManager.BATTERY_PLUGGED_USB
    }

    fun findUsbTetheringAddress(): String? {
        val interfaces = NetworkInterface.getNetworkInterfaces()?.toList().orEmpty()
        for (iface in interfaces) {
            val name = iface.name.lowercase()
            if (name.startsWith("rndis") || name.startsWith("usb")) {
                val addr = iface.inetAddresses.toList().firstOrNull { !it.isLoopbackAddress }
                if (addr != null) {
                    return addr.hostAddress
                }
            }
        }
        return null
    }
}
