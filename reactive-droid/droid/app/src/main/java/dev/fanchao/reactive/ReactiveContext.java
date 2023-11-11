package dev.fanchao.reactive;

import android.app.Activity;
import android.app.Application;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.os.Message;
import android.util.Log;

import java.lang.reflect.Proxy;

public class ReactiveContext {
    private final Activity activity;
    long nativeInstance;
    private final Handler handler = new Handler(Looper.getMainLooper()) {
        @Override
        public void handleMessage(Message msg) {
            if (msg.what == MSG_FRAME) {
                removeMessages(msg.what);
                handleFrame(nativeInstance);
            } else {
                super.handleMessage(msg);
            }
        }
    };

    public ReactiveContext(Activity activity, Bundle state) {
        this.activity = activity;

        Application.ActivityLifecycleCallbacks callbacks = new Application.ActivityLifecycleCallbacks() {
            @Override
            public void onActivityCreated(Activity activity, Bundle savedInstanceState) {
            }

            @Override
            public void onActivityStarted(Activity activity) {
                if (activity == ReactiveContext.this.activity) {
                    onStart(nativeInstance);
                }
            }

            @Override
            public void onActivityResumed(Activity activity) {
                if (activity == ReactiveContext.this.activity) {
                    onResume(nativeInstance);
                }
            }

            @Override
            public void onActivityPaused(Activity activity) {
                if (activity == ReactiveContext.this.activity) {
                    onPause(nativeInstance);
                }
            }

            @Override
            public void onActivityStopped(Activity activity) {
                if (activity == ReactiveContext.this.activity) {
                    onStop(nativeInstance);
                }
            }

            @Override
            public void onActivitySaveInstanceState(Activity activity, Bundle outState) {
                if (activity == ReactiveContext.this.activity) {
                    onSaveInstance(nativeInstance, outState);
                }
            }

            @Override
            public void onActivityDestroyed(Activity activity) {
                if (activity == ReactiveContext.this.activity) {
                    onDestroy(nativeInstance);
                    activity.getApplication().unregisterActivityLifecycleCallbacks(this);
                    handler.removeCallbacksAndMessages(null);
                    nativeInstance = 0L;
                }
            }
        };
        activity.getApplication().registerActivityLifecycleCallbacks(callbacks);
        nativeInstance = onCreate(state);
    }


    native long onCreate(Bundle state);

    native void onSaveInstance(long nativeInstance, Bundle state);

    native void onStart(long nativeInstance);

    native void onResume(long nativeInstance);

    native void onPause(long nativeInstance);

    native void onStop(long nativeInstance);

    native void onDestroy(long nativeInstance);

    native void handleFrame(long nativeInstance);

    static final int MSG_FRAME = 1;

    void requestFrame() {
        handler.sendEmptyMessage(MSG_FRAME);
    }

    void requestDelayedFrame(long delayMillis) {
        handler.sendEmptyMessageDelayed(MSG_FRAME, delayMillis);
    }

    void clearFrameRequests() {
        handler.removeMessages(MSG_FRAME);
    }

    static Object requestProxy(String interfaceName, long nativeData) throws ClassNotFoundException {
        Class<?> i = Class.forName(interfaceName);
        return Proxy.newProxyInstance(i.getClassLoader(),
                new Class<?>[]{i},
                (proxy, method, args) -> onProxyCalled(nativeData, proxy, method.getName(), args)
        );
    }

    static native Object onProxyCalled(
            long nativeData,
            Object proxy,
            String methodName,
            Object[] args
    );

    static boolean shouldLog(int level) {
        return Log.isLoggable("reactive", level);
    }

    static void log(int level, String message) {
        Log.println(level, "reactive", message);
    }

    static {
        System.loadLibrary("reactive_droid");
    }
}
