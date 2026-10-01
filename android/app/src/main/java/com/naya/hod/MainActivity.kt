package com.naya.hod

import android.Manifest
import android.app.Activity
import android.content.Intent
import android.content.pm.PackageManager
import android.graphics.Color
import android.os.Build
import android.os.Bundle
import android.view.Gravity
import android.widget.Button
import android.widget.LinearLayout
import android.widget.TextView

/** 시작/정지 버튼과 상태 표시만 있는 화면. 실제 일은 AudioService가 한다. */
class MainActivity : Activity() {

    private lateinit var statusView: TextView
    private lateinit var button: Button

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
            text = "PC 앱에서 '공유 시작'을 누른 뒤 아래 버튼을 누르세요."
            setTextColor(Color.GRAY)
            setPadding(0, 16, 0, 48)
        }
        button = Button(this).apply { setOnClickListener { toggle() } }
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
            addView(button)
            addView(statusView)
        })
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
        button.text = if (AudioService.isRunning) "정지" else "듣기 시작"
    }

    private fun toggle() {
        if (AudioService.isRunning) {
            startService(Intent(this, AudioService::class.java).setAction(AudioService.ACTION_STOP))
            return
        }
        val missing = permissions.filter { checkSelfPermission(it) != PackageManager.PERMISSION_GRANTED }
        if (missing.isNotEmpty()) {
            requestPermissions(missing.toTypedArray(), 1)
            return
        }
        startForegroundService(Intent(this, AudioService::class.java).setAction(AudioService.ACTION_START))
    }

    override fun onRequestPermissionsResult(code: Int, perms: Array<out String>, results: IntArray) {
        super.onRequestPermissionsResult(code, perms, results)
        if (results.isNotEmpty() && results.all { it == PackageManager.PERMISSION_GRANTED }) {
            toggle()
        } else {
            render("블루투스·알림 권한이 있어야 들을 수 있어요")
        }
    }
}
