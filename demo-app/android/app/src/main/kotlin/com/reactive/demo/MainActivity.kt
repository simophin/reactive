package com.reactive.demo

import android.app.Activity
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.view.Gravity
import android.widget.LinearLayout
import android.widget.TextView
import com.reactive.ReactiveScope

class MainActivity : Activity() {

    private lateinit var scope: ReactiveScope
    private lateinit var statusText: TextView
    private val handler = Handler(Looper.getMainLooper())

    private val tick = object : Runnable {
        private var count = 0
        override fun run() {
            scope.tick()
            statusText.text = "Tick #${++count}"
            handler.postDelayed(this, 16)
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        statusText = TextView(this).apply { textSize = 20f }
        setContentView(LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            gravity = Gravity.CENTER
            addView(statusText)
        })

        scope = ReactiveScope.create()
        handler.post(tick)
    }

    override fun onDestroy() {
        super.onDestroy()
        handler.removeCallbacks(tick)
        scope.destroy()
    }
}
