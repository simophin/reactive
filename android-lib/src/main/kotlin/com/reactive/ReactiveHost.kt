package com.reactive

import android.os.Handler
import android.os.Looper
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier

/**
 * Owns a Rust reactive scope and the [RootNode] its component tree renders into.
 *
 * The Rust library must declare its entry point with `ui_core::android_main!`.
 */
class ReactiveHost private constructor() {
    val root = RootNode()

    private var ptr = 0L
    private val handler = Handler(Looper.getMainLooper())
    private val tick = Runnable { if (ptr != 0L) nativeTick(ptr) }

    /** Called by Rust (from any thread) when signals are dirty or a future woke up. */
    @Suppress("unused")
    fun scheduleTick() {
        handler.post(tick)
    }

    fun destroy() {
        handler.removeCallbacks(tick)
        if (ptr != 0L) {
            nativeDestroy(ptr)
            ptr = 0L
        }
    }

    companion object {
        fun create(libName: String = "reactive_android"): ReactiveHost {
            System.loadLibrary(libName)
            val host = ReactiveHost()
            host.ptr = nativeCreate(host)
            return host
        }

        @JvmStatic private external fun nativeCreate(host: ReactiveHost): Long
        @JvmStatic private external fun nativeTick(ptr: Long)
        @JvmStatic private external fun nativeDestroy(ptr: Long)
    }
}

/** Renders the Rust component tree owned by [host]. */
@Composable
fun ReactiveContent(host: ReactiveHost, modifier: Modifier = Modifier.fillMaxSize()) {
    host.root.Content(modifier)
}
