package dev.fanchao.reactive;

import android.app.Activity;
import android.os.Bundle;


public class MainActivity extends Activity {

    ReactiveContext context;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);

        context = new ReactiveContext(this, savedInstanceState);
    }

    @Override
    protected void onDestroy() {
        super.onDestroy();

        context = null;
    }
}
