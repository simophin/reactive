package com.reactive

class ReactiveScope private constructor(private var ptr: Long) {

    fun tick() {
        check(ptr != 0L) { "ReactiveScope has been destroyed" }
        nativeTick(ptr)
    }

    fun destroy() {
        if (ptr != 0L) {
            nativeDestroy(ptr)
            ptr = 0L
        }
    }

    companion object {
        init {
            System.loadLibrary("reactive_android")
        }

        fun create(): ReactiveScope = ReactiveScope(nativeCreate())

        @JvmStatic private external fun nativeCreate(): Long
        @JvmStatic private external fun nativeDestroy(ptr: Long)
        @JvmStatic private external fun nativeTick(ptr: Long)
    }
}
