package com.joat917.diary2day

import android.content.Intent
import android.net.Uri
import android.os.Bundle
import android.os.Parcelable
import android.provider.OpenableColumns
import android.webkit.JavascriptInterface
import android.webkit.WebView
import androidx.activity.OnBackPressedCallback
import androidx.activity.enableEdgeToEdge
import androidx.core.view.ViewCompat
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.util.UUID
import java.util.concurrent.Executors

class MainActivity : TauriActivity() {
  private val shareQueue = Executors.newSingleThreadExecutor()
  private var webView: WebView? = null
  @Volatile private var insetTopDp = 0f
  @Volatile private var insetRightDp = 0f
  @Volatile private var insetBottomDp = 0f
  @Volatile private var insetLeftDp = 0f
  @Volatile private var insetImeDp = 0f

  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
    enqueueShare(intent)
  }

  override fun onWebViewCreate(webView: WebView) {
    super.onWebViewCreate(webView)
    this.webView = webView
    // Tauri registers its own back callback before the webview exists; registering here keeps
    // ours most recent (dispatch is LIFO) so layers and routes are consumed before it.
    onBackPressedDispatcher.addCallback(
      this,
      object : OnBackPressedCallback(true) {
        override fun handleOnBackPressed() {
          webView.evaluateJavascript("window.ddBack ? window.ddBack() : false") { result ->
            if (result != "true") finish()
          }
        }
      },
    )
    webView.addJavascriptInterface(InsetsBridge(), "ddInsets")
    ViewCompat.setOnApplyWindowInsetsListener(webView) { view, insets ->
      pushInsets(webView, insets)
      insets
    }
    webView.post {
      val insets = ViewCompat.getRootWindowInsets(webView)
      if (insets != null) pushInsets(webView, insets)
      ViewCompat.requestApplyInsets(webView)
    }
  }

  private fun pushInsets(webView: WebView, insets: WindowInsetsCompat) {
    val bars = insets.getInsets(
      WindowInsetsCompat.Type.systemBars() or WindowInsetsCompat.Type.displayCutout(),
    )
    val imePx =
      if (insets.isVisible(WindowInsetsCompat.Type.ime())) {
        insets.getInsets(WindowInsetsCompat.Type.ime()).bottom
      } else {
        0
      }
    val density = resources.displayMetrics.density.takeIf { it > 0f } ?: 1f
    val top = bars.top / density
    val right = bars.right / density
    val bottom = bars.bottom / density
    val left = bars.left / density
    val ime = imePx / density
    if (top == insetTopDp &&
      right == insetRightDp &&
      bottom == insetBottomDp &&
      left == insetLeftDp &&
      ime == insetImeDp
    ) {
      return
    }
    insetTopDp = top
    insetRightDp = right
    insetBottomDp = bottom
    insetLeftDp = left
    insetImeDp = ime
    webView.post {
      webView.evaluateJavascript(
        "window.ddApplyInsets && window.ddApplyInsets($top, $right, $bottom, $left, $ime)",
        null,
      )
    }
  }

  private inner class InsetsBridge {
    @JavascriptInterface
    fun read(): String =
      "$insetTopDp,$insetRightDp,$insetBottomDp,$insetLeftDp,$insetImeDp"

    @JavascriptInterface
    fun setDark(dark: Boolean) {
      val view = webView ?: return
      view.post {
        WindowCompat.getInsetsController(window, view).apply {
          isAppearanceLightStatusBars = !dark
          isAppearanceLightNavigationBars = !dark
        }
      }
    }
  }

  override fun onNewIntent(intent: Intent) {
    super.onNewIntent(intent)
    setIntent(intent)
    enqueueShare(intent)
  }

  private fun enqueueShare(intent: Intent?) {
    if (intent == null) return
    val action = intent.action
    if (action != Intent.ACTION_SEND && action != Intent.ACTION_SEND_MULTIPLE) return
    shareQueue.execute { stageShare(intent) }
  }

  @Suppress("DEPRECATION")
  private fun stageShare(intent: Intent) {
    val action = intent.action
    val text = intent.getCharSequenceExtra(Intent.EXTRA_TEXT)?.toString()?.trim().orEmpty()
    val uris = mutableListOf<Uri>()
    if (action == Intent.ACTION_SEND) {
      extractUri(intent.getParcelableExtra(Intent.EXTRA_STREAM))?.let { uris.add(it) }
    } else {
      val streams: ArrayList<Parcelable>? =
        intent.getParcelableArrayListExtra(Intent.EXTRA_STREAM)
      streams?.forEach { value ->
        extractUri(value)?.let { uris.add(it) }
      }
    }
    if (uris.isEmpty()) {
      intent.clipData?.let { clip ->
        for (index in 0 until clip.itemCount) {
          clip.getItemAt(index).uri?.let { uris.add(it) }
        }
      }
    }
    if (text.isEmpty() && uris.isEmpty()) return

    val inbox = File(cacheDir, "share-inbox")
    if (!inbox.exists() && !inbox.mkdirs()) return

    val files = JSONArray()
    for (uri in uris) {
      val staged = stageFile(inbox, uri, intent.type)
      if (staged != null) files.put(staged)
    }
    if (text.isEmpty() && files.length() == 0) return

    val pending = File(inbox, "pending.json")
    val queue = try {
      if (pending.exists()) JSONArray(pending.readText()) else JSONArray()
    } catch (_: Exception) {
      JSONArray()
    }
    queue.put(
      JSONObject().apply {
        put("text", text)
        put("files", files)
      },
    )
    val temp = File(inbox, "pending.tmp")
    try {
      temp.writeText(queue.toString())
      if (!temp.renameTo(pending)) {
        pending.writeText(queue.toString())
        temp.delete()
      }
    } catch (_: Exception) {
      temp.delete()
    }
  }

  @Suppress("DEPRECATION")
  private fun extractUri(value: Any?): Uri? = when (value) {
    is Uri -> value
    is String -> Uri.parse(value)
    else -> null
  }

  private fun stageFile(inbox: File, uri: Uri, fallbackMime: String?): JSONObject? {
    val name = displayName(uri) ?: "shared"
    val mime = contentResolver.getType(uri) ?: fallbackMime ?: ""
    val ext = name
      .substringAfterLast('.', "")
      .lowercase()
      .takeIf { it.length in 1..5 && it.all(Char::isLetterOrDigit) }
      ?: mimeExt(mime)
    val target = File(inbox, "${UUID.randomUUID()}.${ext.ifEmpty { "bin" }}")
    try {
      val input = contentResolver.openInputStream(uri) ?: return null
      input.use {
        target.outputStream().use { output ->
          val buffer = ByteArray(64 * 1024)
          while (true) {
            val read = it.read(buffer)
            if (read < 0) break
            output.write(buffer, 0, read)
          }
        }
      }
    } catch (_: Exception) {
      target.delete()
      return null
    }
    return JSONObject().apply {
      put("path", target.absolutePath)
      put("name", name)
      put("mime", mime)
    }
  }

  private fun displayName(uri: Uri): String? {
    try {
      contentResolver
        .query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)
        ?.use { cursor ->
          val index = cursor.getColumnIndex(OpenableColumns.DISPLAY_NAME)
          if (index >= 0 && cursor.moveToFirst()) {
            return cursor.getString(index)
          }
        }
    } catch (_: Exception) {
    }
    return uri.lastPathSegment?.substringAfterLast('/')
  }

  private fun mimeExt(mime: String): String {
    val type = mime.substringBefore(';').trim().lowercase()
    if (!type.contains('/')) return ""
    val ext = type.substringAfter('/')
    return if (ext == "svg+xml" || ext == "x-matroska") "" else ext
  }
}