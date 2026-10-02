package com.reactive

import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize

/**
 * A UI object owned by Rust.
 *
 * Rust creates nodes and calls their setters; setters write Compose snapshot state, and
 * [Content] reads that state, so Compose recomposes only what Rust changed.
 */
abstract class Node {
    @Composable
    abstract fun Content(modifier: Modifier)
}

/** Holds the single top-level node of a Rust component tree, which fills it like a window. */
class RootNode : Node() {
    var child by mutableStateOf<Node?>(null)

    @Composable
    override fun Content(modifier: Modifier) {
        Box(modifier) { child?.Content(Modifier.fillMaxSize()) }
    }
}
