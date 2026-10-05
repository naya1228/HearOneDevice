package com.naya.hod

import android.Manifest
import android.app.Activity
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.graphics.Color
import android.os.Build
import android.os.Bundle
import android.view.Gravity
import android.widget.Button
import android.widget.LinearLayout
import android.widget.TextView
import com.google.mlkit.vision.barcode.common.Barcode
import com.google.mlkit.vision.codescanner.GmsBarcodeScannerOptions
import com.google.mlkit.vision.codescanner.GmsBarcodeScanning

/**
 * 시작/정지, QR 스캔 버튼과 상태 표시만 있는 화면. 실제 일은 AudioService가 한다.
 * PC의 QR(hearone://connect?id=...)은 앱 안 스캐너로 찍거나, 폰 카메라로 찍어 이 화면을 열 수 있다.
 */
class MainActivity : Activity() {

    companion object {
        private const val PREFS = "hod"
        private const val KEY_LAST_PC = "last_pc"

        fun saveLastPc(ctx: Context, id: String) =
            ctx.getSharedPreferences(PREFS, MODE_PRIVATE).edit().putString(KEY_LAST_PC, id).apply()

        fun lastPc(ctx: Context): String? =
            ctx.getSharedPreferences(PREFS, MODE_PRIVATE).getString(KEY_LAST_PC, null)
    }

    private lateinit var statusView: TextView
    private lateinit var listenButton: Button

    // 권한을 받는 동안 기다리는 연결 요청 (null 이면 마지막 PC)
    private var pendingId: String? = null

    private val permissions: Array<String>
        get() = buildList {
            if (Build.VERSION.SDK_INT >= 31) {
                add(Manifest.permission.BLUETOOTH_SCAN)
                add(Manifest.permission.BLUETOOTH_CONNECT)
            } else {
                add(Manifest.permission.ACCESS_FINE_LOCATION)
            }
            if (Build.VERSION.SDK_INT >= 33) add(Manifest.permission.POST_NOTIFICATIONS)
        }.toTypedArray()

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val title = TextView(this).apply {
            text = "HearOneDevice"
            textSize = 28f
            setTextColor(Color.WHITE)
        }
        val hint = TextView(this).apply {
            text = "PC 앱에서 '공유 시작'을 누르고 화면의 QR을 찍으세요."
            setTextColor(Color.GRAY)
            setPadding(0, 16, 0, 48)
        }
        val scanButton = Button(this).apply {
            text = "QR 스캔해서 연결"
            setOnClickListener { scanQr() }
        }
        listenButton = Button(this).apply { setOnClickListener { toggle() } }
        statusView = TextView(this).apply {
            setTextColor(Color.rgb(0xFD, 0x60, 0x00))
            setPadding(0, 48, 0, 0)
            gravity = Gravity.CENTER
        }
        setContentView(LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            gravity = Gravity.CENTER
            setPadding(48, 48, 48, 48)
            setBackgroundColor(Color.rgb(0x1F, 0x1F, 0x1E))
            addView(title)
            addView(hint)
            addView(scanButton)
            addView(listenButton, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT,
            ).apply { topMargin = (16 * resources.displayMetrics.density).toInt() })
            addView(statusView)
        })

        handleLink(intent)
    }

    // 앱이 이미 열린 상태에서 카메라로 QR을 찍은 경우
    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        handleLink(intent)
    }

    override fun onResume() {
        super.onResume()
        AudioService.onStatus = { s -> runOnUiThread { render(s) } }
        render(AudioService.status)
    }

    override fun onPause() {
        AudioService.onStatus = null
        super.onPause()
    }

    private fun render(status: String) {
        statusView.text = status
        val last = lastPc(this)
        listenButton.text = when {
            AudioService.isRunning -> "정지"
            last != null -> "마지막 PC($last)로 듣기"
            else -> "가까운 PC로 듣기"
        }
    }

    private fun handleLink(intent: Intent?) {
        val id = Protocol.parseLink(intent?.dataString) ?: return
        intent?.data = null // 화면 회전 등으로 다시 처리되지 않게
        connect(id)
    }

    private fun scanQr() {
        val options = GmsBarcodeScannerOptions.Builder()
            .setBarcodeFormats(Barcode.FORMAT_QR_CODE)
            .build()
        GmsBarcodeScanning.getClient(this, options).startScan()
            .addOnSuccessListener { code ->
                val id = Protocol.parseLink(code.rawValue)
                if (id != null) connect(id) else render("HearOneDevice QR이 아니에요")
            }
            .addOnFailureListener { render("QR 스캐너를 열 수 없어요: ${it.message}") }
    }

    private fun toggle() {
        if (AudioService.isRunning) {
            startService(Intent(this, AudioService::class.java).setAction(AudioService.ACTION_STOP))
        } else {
            connect(lastPc(this))
        }
    }

    /** id가 null이면 처음 발견한 PC에 연결 */
    private fun connect(id: String?) {
        val missing = permissions.filter { checkSelfPermission(it) != PackageManager.PERMISSION_GRANTED }
        if (missing.isNotEmpty()) {
            pendingId = id
            requestPermissions(missing.toTypedArray(), 1)
            return
        }
        if (id != null) saveLastPc(this, id)
        startForegroundService(
            Intent(this, AudioService::class.java)
                .setAction(AudioService.ACTION_START)
                .putExtra(AudioService.EXTRA_ID, id)
        )
    }

    override fun onRequestPermissionsResult(code: Int, perms: Array<out String>, results: IntArray) {
        super.onRequestPermissionsResult(code, perms, results)
        if (results.isNotEmpty() && results.all { it == PackageManager.PERMISSION_GRANTED }) {
            connect(pendingId)
        } else {
            render("블루투스·알림 권한이 있어야 들을 수 있어요")
        }
    }
}
