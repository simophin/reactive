package com.reactive;

import android.app.Activity;
import android.content.pm.ActivityInfo;
import android.content.pm.PackageManager;
import android.os.Bundle;

/**
 * Hosts a Rust component tree. The native library is named by the activity's
 * {@code com.reactive.lib_name} meta-data, like {@code android.app.NativeActivity}:
 *
 * <pre>{@code
 * <activity android:name="com.reactive.ReactiveActivity" android:exported="true">
 *     <meta-data android:name="com.reactive.lib_name" android:value="my_app" />
 * </activity>
 * }</pre>
 *
 * The library declares its entry point with {@code ui_core::android_main!}.
 */
public class ReactiveActivity extends Activity {
    public static final String META_LIB_NAME = "com.reactive.lib_name";

    private long nativeHandle;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        System.loadLibrary(libraryName());
        nativeHandle = nativeCreate();
    }

    @Override
    protected void onDestroy() {
        long handle = nativeHandle;
        nativeHandle = 0;
        if (handle != 0) {
            nativeDestroy(handle);
        }
        super.onDestroy();
    }

    private String libraryName() {
        try {
            ActivityInfo info = getPackageManager()
                    .getActivityInfo(getComponentName(), PackageManager.GET_META_DATA);
            String name = info.metaData != null ? info.metaData.getString(META_LIB_NAME) : null;
            if (name == null) {
                throw new IllegalStateException("Missing <meta-data android:name=\"" + META_LIB_NAME + "\">");
            }
            return name;
        } catch (PackageManager.NameNotFoundException e) {
            throw new IllegalStateException(e);
        }
    }

    private native long nativeCreate();

    private native void nativeDestroy(long handle);
}
