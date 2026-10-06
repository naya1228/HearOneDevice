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
import android.view.View
import android.widget.Button
import android.widget.LinearLayout
import android.widget.TextView
import android.widget.Toast
import com.google.mlkit.vision.barcode.common.Barcode
import com.google.mlkit.vision.codescanner.GmsBarcodeScannerOptions
import com.google.mlkit.vision.codescanner.GmsBarcodeScanning

/**
 * 시작/정지, QR 스캔, PC 연결 기록 버튼과 상태 표시만 있는 화면. 실제 일은 AudioService가 한다.
 * PC의 QR(docs/PROTOCOL.md 1절)은 앱 안 스캐너로 찍거나, 폰 카메라로 찍어 이 화면을 열 수 있다.
 */
class MainActivity : Activity() {

    companion object {
        private const val REQ_HISTORY = 1
    }

    private lateinit var statusView: TextView
    private lateinit var statsView: TextView
    private lateinit var listenButton: Button

    // 권한을 받는 동안 기다리는 연결 요청
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
            setOnClickListener {
                val on = DevMode.onTap(this@MainActivity) ?: return@setOnClickListener
                Toast.makeText(this@MainActivity, if (on) "개발 모드 켜짐" else "개발 모드 꺼짐", Toast.LENGTH_SHORT).show()
                renderStats(AudioService.stats)
            }
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
        val historyButton = Button(this).apply {
            text = "PC 연결 기록"
            @Suppress("DEPRECATION")
            setOnClickListener { startActivityForResult(Intent(this@MainActivity, HistoryActivity::class.java), REQ_HISTORY) }
        }
        statusView = TextView(this).apply {
            setTextColor(Color.rgb(0xFD, 0x60, 0x00))
            setPadding(0, 48, 0, 0)
            gravity = Gravity.CENTER
        }
        statsView = TextView(this).apply {
            setTextColor(Color.GRAY)
            textSize = 12f
            setPadding(0, 16, 0, 0)
            gravity = Gravity.CENTER
        }
        setContentView(LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            gravity = Gravity.CENTER
            setPadding(48, 48, 48, 48)
            setBackgroundColor(Color.rgb(0x1F, 0x1F, 0x1E))
            addView(title)
            addView(hint)
            val size = (240 * resources.displayMetrics.density).toInt()
            addView(CircleVisualizer(this@MainActivity), LinearLayout.LayoutParams(size, size).apply {
                bottomMargin = (24 * resources.displayMetrics.density).toInt()
            })
            addView(scanButton)
            addView(listenButton, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT,
            ).apply { topMargin = (16 * resources.displayMetrics.density).toInt() })
            addView(historyButton, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT,
            ))
            addView(statusView)
            addView(statsView)
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
        AudioService.onStats = { s -> runOnUiThread { renderStats(s) } }
        render(AudioService.status)
        renderStats(AudioService.stats)
    }

    override fun onPause() {
        AudioService.onStatus = null
        AudioService.onStats = null
        super.onPause()
    }

    // 상세 숫자는 개발 모드일 때만
    private fun renderStats(stats: String?) {
        val show = stats != null && DevMode.isOn(this)
        statsView.text = if (show) stats else ""
        statsView.visibility = if (show) View.VISIBLE else View.GONE
    }

    private fun render(status: String) {
        statusView.text = status
        // 기록이 없으면 버튼을 숨김 (QR로 먼저 연결해야 기록이 생김. 아무 PC나 찾지 않는다)
        val last = SavedPcs.last(this)
        listenButton.visibility = if (AudioService.isRunning || last != null) View.VISIBLE else View.GONE
        listenButton.text = if (AudioService.isRunning) "정지" else "마지막 PC(${last?.label})로 듣기"
    }

    private fun handleLink(intent: Intent?) {
        val link = Protocol.parseLink(intent?.dataString) ?: return
        intent?.data = null // 화면 회전 등으로 다시 처리되지 않게
        connectLink(link)
    }

    // QR로 받은 PC를 기록에 넣고 연결
    private fun connectLink(link: Protocol.Link) {
        SavedPcs.remember(this, link.id, link.name)
        connect(link.id)
    }

    private fun scanQr() {
        val options = GmsBarcodeScannerOptions.Builder()
            .setBarcodeFormats(Barcode.FORMAT_QR_CODE)
            .build()
        GmsBarcodeScanning.getClient(this, options).startScan()
            .addOnSuccessListener { code ->
                val link = Protocol.parseLink(code.rawValue)
                if (link != null) connectLink(link) else render("HearOneDevice QR이 아니에요")
            }
            .addOnFailureListener { render("QR 스캐너를 열 수 없어요: ${it.message}") }
    }

    private fun toggle() {
        if (AudioService.isRunning) {
            startService(Intent(this, AudioService::class.java).setAction(AudioService.ACTION_STOP))
        } else {
            SavedPcs.last(this)?.let { connect(it.id) }
        }
    }

    // PC 연결 기록에서 고른 PC로 연결
    @Deprecated("Activity 기본 API")
    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        @Suppress("DEPRECATION")
        super.onActivityResult(requestCode, resultCode, data)
        val id = data?.getStringExtra(HistoryActivity.EXTRA_ID)
        if (requestCode == REQ_HISTORY && resultCode == RESULT_OK && id != null) connect(id)
    }

    private fun connect(id: String) {
        val missing = permissions.filter { checkSelfPermission(it) != PackageManager.PERMISSION_GRANTED }
        if (missing.isNotEmpty()) {
            pendingId = id
            requestPermissions(missing.toTypedArray(), 1)
            return
        }
        SavedPcs.remember(this, id, null)
        startForegroundService(
            Intent(this, AudioService::class.java)
                .setAction(AudioService.ACTION_START)
                .putExtra(AudioService.EXTRA_ID, id)
        )
    }

    override fun onRequestPermissionsResult(code: Int, perms: Array<out String>, results: IntArray) {
        super.onRequestPermissionsResult(code, perms, results)
        if (results.isNotEmpty() && results.all { it == PackageManager.PERMISSION_GRANTED }) {
            pendingId?.let { connect(it) }
        } else {
            render("블루투스·알림 권한이 있어야 들을 수 있어요")
        }
    }
}
