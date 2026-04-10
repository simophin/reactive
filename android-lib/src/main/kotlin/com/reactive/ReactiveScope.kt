package com.reactive

import android.app.Activity
import android.os.Handler
import android.os.Looper

class ReactiveScope private constructor(private var ptr: Long) {

    fun tick() {
        check(ptr != 0L) { "ReactiveScope has been destroyed" }
        nativeTick(ptr)
    }

    fun attach(activity: Activity) {
        check(ptr != 0L) { "ReactiveScope has been destroyed" }
        activeScope = this
        nativeAttachActivity(ptr, activity)
    }

    fun destroy() {
        if (ptr != 0L) {
            if (activeScope === this) {
                activeScope = null
            }
            nativeDestroy(ptr)
            ptr = 0L
        }
    }

    companion object {
        private val mainHandler = Handler(Looper.getMainLooper())
        private var activeScope: ReactiveScope? = null

        init {
            System.loadLibrary("reactive_android")
        }

        fun create(): ReactiveScope = ReactiveScope(nativeCreate())

        @JvmStatic
        fun scheduleTick() {
            val scope = activeScope ?: return
            mainHandler.post { scope.tick() }
        }

        @JvmStatic private external fun nativeCreate(): Long
        @JvmStatic private external fun nativeAttachActivity(ptr: Long, activity: Activity)
        @JvmStatic private external fun nativeDestroy(ptr: Long)
        @JvmStatic private external fun nativeTick(ptr: Long)
    }
}
