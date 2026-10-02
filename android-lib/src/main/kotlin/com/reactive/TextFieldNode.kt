package com.reactive

import androidx.compose.foundation.text.input.TextFieldState
import androidx.compose.foundation.text.input.setTextAndPlaceCursorAtEnd
import androidx.compose.material3.LocalTextStyle
import androidx.compose.material3.OutlinedTextField
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.TextRange
import androidx.compose.ui.unit.sp

/**
 * Editable text backed by a [TextFieldState] that lives as long as the node.
 *
 * User edits are reported through [onChange] as `invoke(selection, text)`, where `selection`
 * packs UTF-16 `start shl 32 or end`, and `text` is null when only the selection moved.
 * Text set from Rust via [setText] is not echoed back.
 */
class TextFieldNode : Node() {
    val state = TextFieldState()
    var fontSize by mutableFloatStateOf(16f)
    var onChange: NativeCallback? = null

    private var lastText = ""
    private var lastSelection = TextRange.Zero

    fun setText(text: String) {
        lastText = text
        state.setTextAndPlaceCursorAtEnd(text)
    }

    @Composable
    override fun Content(modifier: Modifier) {
        LaunchedEffect(state) {
            snapshotFlow { state.text.toString() to state.selection }.collect { (text, selection) ->
                val textChanged = text != lastText
                if (!textChanged && selection == lastSelection) return@collect
                lastText = text
                lastSelection = selection
                onChange?.invoke(
                    (selection.start.toLong() shl 32) or selection.end.toLong(),
                    if (textChanged) text else null,
                )
            }
        }
        OutlinedTextField(
            state,
            modifier,
            textStyle = LocalTextStyle.current.copy(fontSize = fontSize.sp),
        )
    }
}
