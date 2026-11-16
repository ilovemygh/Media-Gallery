package io.mediagallery.plugins.bluetooth

import android.annotation.SuppressLint
import android.bluetooth.BluetoothAdapter
import android.bluetooth.BluetoothDevice
import android.bluetooth.BluetoothManager
import android.content.Context
import java.util.UUID

/**
 * Bluetooth Classic RFCOMM / SPP fallback channel plugin (PRD Sec 8 & D5).
 */
class BluetoothTransportPlugin(private val context: Context) {
    companion object {
        val SERVICE_UUID: UUID = UUID.fromString("8a9e3b82-5f3b-4c6f-a35e-4e65c879f443")
    }

    fun isBluetoothReady(): Boolean {
        val manager = context.getSystemService(Context.BLUETOOTH_SERVICE) as? BluetoothManager
        val adapter: BluetoothAdapter? = manager?.adapter
        return adapter != null && adapter.isEnabled
    }

    @SuppressLint("MissingPermission")
    fun sendChunkPacket(deviceAddress: String, bytes: ByteArray): Boolean {
        val manager = context.getSystemService(Context.BLUETOOTH_SERVICE) as? BluetoothManager ?: return false
        val device: BluetoothDevice = manager.adapter?.getRemoteDevice(deviceAddress) ?: return false
        device.createRfcommSocketToServiceRecord(SERVICE_UUID).use { socket ->
            socket.connect()
            socket.outputStream.write(bytes)
            socket.outputStream.flush()
        }
        return true
    }
}
