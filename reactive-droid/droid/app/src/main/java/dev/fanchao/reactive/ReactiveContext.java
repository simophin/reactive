package dev.fanchao.reactive;

import android.app.Activity;
import android.app.Application;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.os.Message;

import java.lang.reflect.InvocationHandler;
import java.lang.reflect.Method;
import java.lang.reflect.Proxy;

public class ReactiveContext {
    private final Activity activity;
    private boolean isActivityStarted = false;
    long nativeInstance;
    private final Handler handler = new Handler(Looper.getMainLooper()) {
        @Override
        public void handleMessage(Message msg) {
            if (msg.what == MSG_FRAME) {
                removeMessages(msg.what);
                if (isActivityStarted && handleFrame(nativeInstance)) {
                    sendEmptyMessage(MSG_FRAME);
                }
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
                    isActivityStarted = true;
                    if (handleFrame(nativeInstance)) {
                        requestFrame();
                    }
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
                    clearFrameRequests();
                    isActivityStarted = false;
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
        nativeInstance = onCreate(state, activity);
    }


    native long onCreate(Bundle state, Activity activity);

    native void onSaveInstance(long nativeInstance, Bundle state);

    native void onStart(long nativeInstance);

    native void onResume(long nativeInstance);

    native void onPause(long nativeInstance);

    native void onStop(long nativeInstance);

    native void onDestroy(long nativeInstance);

    native boolean handleFrame(long nativeInstance);

    static final int MSG_FRAME = 1;

    void requestFrame() {
        handler.sendEmptyMessage(MSG_FRAME);
    }


    void clearFrameRequests() {
        handler.removeMessages(MSG_FRAME);
    }

    static Object requestProxy(String interfaceName, final long nativeData) throws ClassNotFoundException {
        Class<?> i = Class.forName(interfaceName);
        return Proxy.newProxyInstance(i.getClassLoader(),
                new Class<?>[]{i},
                new InvocationHandler() {
                    @Override
                    public Object invoke(Object proxy, Method method, Object[] args) throws Throwable {
                        final String methodName = method.getName();

                        if (methodName.equals("hashCode") && (args == null || args.length == 0)) {
                            return System.identityHashCode(proxy);
                        }

                        if (methodName.equals("equals") && (args != null && args.length == 1 && args[0].getClass() == Object.class)) {
                            return proxy == args[0];
                        }

                        if (methodName.equals("toString") && (args == null || args.length == 0)) {
                            return proxy.getClass().getName() + '@' + Integer.toHexString(System.identityHashCode(proxy));
                        }

                        return onProxyCalled(nativeData, proxy, methodName, args);
                    }

                    @Override
                    protected void finalize() throws Throwable {
                        super.finalize();
                        onProxyDestroyed(nativeData);
                    }
                }
        );
    }

    static native Object onProxyCalled(
            long nativeData,
            Object proxy,
            String methodName,
            Object[] args
    );

    static native void onProxyDestroyed(long nativeData);

    static {
        System.loadLibrary("reactive_droid");
    }
}
