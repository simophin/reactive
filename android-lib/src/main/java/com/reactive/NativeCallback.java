package com.reactive;

import android.view.View;

/** Forwards listener events to a Rust closure. */
public final class NativeCallback implements View.OnClickListener {
    private long handle;

    private NativeCallback(long handle) {
        this.handle = handle;
    }

    @Override
    public void onClick(View view) {
        if (handle != 0) {
            nativeInvoke(handle);
        }
    }

    /** Frees the Rust closure; later events are ignored. */
    public void release() {
        long h = handle;
        handle = 0;
        if (h != 0) {
            nativeRelease(h);
        }
    }

    private static native void nativeInvoke(long handle);

    private static native void nativeRelease(long handle);
}
