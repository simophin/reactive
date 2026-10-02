package com.reactive.demo

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.ui.Modifier
import com.reactive.ReactiveContent
import com.reactive.ReactiveHost

class MainActivity : ComponentActivity() {
    private lateinit var host: ReactiveHost

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        host = ReactiveHost.create()
        setContent {
            MaterialTheme {
                Surface(Modifier.fillMaxSize()) {
                    ReactiveContent(host, Modifier.fillMaxSize().safeDrawingPadding())
                }
            }
        }
    }

    override fun onDestroy() {
        host.destroy()
        super.onDestroy()
    }
}
