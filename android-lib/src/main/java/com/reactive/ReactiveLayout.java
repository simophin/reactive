package com.reactive;

import android.content.Context;
import android.view.ViewGroup;

/**
 * A ViewGroup whose measure and layout passes are delegated to a Rust layout
 * object. It knows nothing about any particular layout algorithm: Flex is one
 * such layout, and any custom container can use the same class.
 */
public final class ReactiveLayout extends ViewGroup {
    /** Owned by Rust; zero when no layout is attached. */
    long nativeHandle;

    public ReactiveLayout(Context context) {
        super(context);
    }

    @Override
    protected void onMeasure(int widthMeasureSpec, int heightMeasureSpec) {
        if (nativeHandle == 0) {
            setMeasuredDimension(
                    resolveSize(0, widthMeasureSpec), resolveSize(0, heightMeasureSpec));
            return;
        }
        long size = nativeMeasure(nativeHandle, widthMeasureSpec, heightMeasureSpec);
        setMeasuredDimension((int) (size >>> 32), (int) size);
    }

    @Override
    protected void onLayout(boolean changed, int left, int top, int right, int bottom) {
        if (nativeHandle != 0) {
            nativeLayout(nativeHandle, right - left, bottom - top);
        }
    }

    /** Returns the measured size packed as {@code width << 32 | height}. */
    private static native long nativeMeasure(long handle, int widthMeasureSpec, int heightMeasureSpec);

    private static native void nativeLayout(long handle, int width, int height);
}
