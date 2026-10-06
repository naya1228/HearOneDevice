package com.naya.hod

import android.app.Activity
import android.app.AlertDialog
import android.content.Intent
import android.graphics.Color
import android.os.Bundle
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView

/**
 * PC 연결 기록 화면. 누르면 그 PC 번호를 MainActivity에 돌려주고(연결은 MainActivity가), 길게 누르면 이름 바꾸기·삭제.
 */
class HistoryActivity : Activity() {

    companion object {
        /** 결과 Intent에 담기는 고른 PC 번호 */
        const val EXTRA_ID = "id"
    }

    private lateinit var list: LinearLayout

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val dp = resources.displayMetrics.density
        list = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL }
        setContentView(LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(48, 48, 48, 48)
            setBackgroundColor(Color.rgb(0x1F, 0x1F, 0x1E))
            addView(TextView(this@HistoryActivity).apply {
                text = "PC 연결 기록"
                textSize = 24f
                setTextColor(Color.WHITE)
                setPadding(0, 0, 0, (16 * dp).toInt())
            })
            addView(TextView(this@HistoryActivity).apply {
                text = "누르면 연결 · 길게 누르면 이름 바꾸기·삭제"
                setTextColor(Color.GRAY)
                setPadding(0, 0, 0, (16 * dp).toInt())
            })
            addView(ScrollView(this@HistoryActivity).apply { addView(list) })
        })
        render()
    }

    private fun render() {
        list.removeAllViews()
        val pcs = SavedPcs.all(this)
        if (pcs.isEmpty()) {
            list.addView(TextView(this).apply {
                text = "아직 없어요. QR로 연결한 PC가 여기에 쌓여요."
                setTextColor(Color.GRAY)
            })
            return
        }
        val dp = resources.displayMetrics.density
        for (pc in pcs) {
            list.addView(TextView(this).apply {
                text = pc.label
                textSize = 18f
                setTextColor(Color.WHITE)
                setPadding(0, (12 * dp).toInt(), 0, (12 * dp).toInt())
                setOnClickListener {
                    setResult(RESULT_OK, Intent().putExtra(EXTRA_ID, pc.id))
                    finish()
                }
                setOnLongClickListener { edit(pc); true }
            })
        }
    }

    private fun edit(pc: SavedPcs.Pc) {
        AlertDialog.Builder(this)
            .setTitle(pc.label)
            .setItems(arrayOf("이름 바꾸기", "삭제")) { _, which ->
                if (which == 0) rename(pc) else {
                    SavedPcs.remove(this, pc.id)
                    render()
                }
            }
            .show()
    }

    private fun rename(pc: SavedPcs.Pc) {
        val input = EditText(this).apply { setText(pc.name); selectAll() }
        AlertDialog.Builder(this)
            .setTitle("이름 바꾸기")
            .setView(input)
            .setPositiveButton("저장") { _, _ ->
                SavedPcs.rename(this, pc.id, input.text.toString())
                render()
            }
            .setNegativeButton("취소", null)
            .show()
    }
}
