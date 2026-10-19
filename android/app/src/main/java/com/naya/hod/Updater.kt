package com.naya.hod

import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.content.pm.PackageInstaller
import android.os.Build
import org.json.JSONObject
import java.net.HttpURLConnection
import java.net.URL
import kotlin.concurrent.thread

// docs/FEATURES.md: 업데이트
object Updater {
    private const val LATEST = "https://api.github.com/repos/naya1228/HearOneDevice/releases/latest"

    private const val CONNECT_TIMEOUT_MS = 10_000
    private const val READ_TIMEOUT_MS = 30_000

    class Release(val version: String, val apkUrl: String)

    fun check(ctx: Context, onResult: (Release?) -> Unit) {
        val current = ctx.packageManager.getPackageInfo(ctx.packageName, 0).versionName ?: return onResult(null)
        thread {
            onResult(runCatching {
                val json = JSONObject(get(LATEST).bufferedReader().use { it.readText() })
                val version = json.getString("tag_name").removePrefix("v")
                val assets = json.getJSONArray("assets")
                val apk = (0 until assets.length()).map { assets.getJSONObject(it) }
                    .firstOrNull { it.getString("name").endsWith(".apk") }
                    ?: return@runCatching null
                if (newer(version, current)) Release(version, apk.getString("browser_download_url")) else null
            }.getOrNull())
        }
    }

    fun install(ctx: Context, release: Release, onError: (String) -> Unit) {
        val app = ctx.applicationContext
        thread {
            runCatching {
                val installer = app.packageManager.packageInstaller
                val params = PackageInstaller.SessionParams(PackageInstaller.SessionParams.MODE_FULL_INSTALL)
                val id = installer.createSession(params)
                installer.openSession(id).use { session ->
                    get(release.apkUrl).use { input ->
                        session.openWrite("app.apk", 0, -1).use { out ->
                            input.copyTo(out)
                            session.fsync(out)
                        }
                    }
                    val flags = PendingIntent.FLAG_UPDATE_CURRENT or
                        (if (Build.VERSION.SDK_INT >= 31) PendingIntent.FLAG_MUTABLE else 0)
                    val result = PendingIntent.getBroadcast(app, 0, Intent(app, UpdateReceiver::class.java), flags)
                    session.commit(result.intentSender)
                }
            }.onFailure { onError(it.message ?: it.toString()) }
        }
    }

    private fun get(url: String) = (URL(url).openConnection() as HttpURLConnection).run {
        connectTimeout = CONNECT_TIMEOUT_MS
        readTimeout = READ_TIMEOUT_MS
        if (responseCode !in 200..299) throw IllegalStateException("HTTP $responseCode")
        inputStream
    }

    fun newer(a: String, b: String): Boolean {
        val x = a.split(".").map { it.toIntOrNull() ?: 0 }
        val y = b.split(".").map { it.toIntOrNull() ?: 0 }
        for (i in 0 until maxOf(x.size, y.size)) {
            val d = x.getOrElse(i) { 0 } - y.getOrElse(i) { 0 }
            if (d != 0) return d > 0
        }
        return false
    }
}
