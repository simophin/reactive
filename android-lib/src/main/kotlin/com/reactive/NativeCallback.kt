package com.reactive

/**
 * The single way Kotlin calls back into a Rust closure.
 *
 * Rust owns the closure and calls [release] when the owning component is disposed, after
 * which late events (e.g. a click queued before disposal) are ignored.
 */
class NativeCallback(private var ptr: Long) {
    fun invoke(value: Long = 0L, arg: Any? = null): Long {
        val p = ptr
        return if (p != 0L) nativeInvoke(p, value, arg) else 0L
    }

    fun release() {
        ptr = 0L
    }

    private external fun nativeInvoke(ptr: Long, value: Long, arg: Any?): Long
}
