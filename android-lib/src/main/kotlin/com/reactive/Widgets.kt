package com.reactive

import android.graphics.Bitmap
import android.graphics.BitmapFactory
import androidx.compose.foundation.Image
import androidx.compose.foundation.layout.Box
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.Slider
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.sp

class TextNode : Node() {
    var text by mutableStateOf("")
    var fontSize by mutableFloatStateOf(14f)

    /** 0 = start, 1 = center, 2 = end. */
    var align by mutableIntStateOf(0)

    @Composable
    override fun Content(modifier: Modifier) {
        Text(
            text,
            modifier,
            fontSize = fontSize.sp,
            textAlign = when (align) {
                1 -> TextAlign.Center
                2 -> TextAlign.End
                else -> TextAlign.Start
            },
        )
    }
}

class ButtonNode : Node() {
    var title by mutableStateOf("")
    var enabled by mutableStateOf(true)
    var onClick: NativeCallback? = null

    @Composable
    override fun Content(modifier: Modifier) {
        Button(onClick = { onClick?.invoke() }, modifier, enabled = enabled) { Text(title) }
    }
}

class SliderNode : Node() {
    var value by mutableFloatStateOf(0f)
    var min by mutableFloatStateOf(0f)
    var max by mutableFloatStateOf(1f)
    var onChange: NativeCallback? = null

    @Composable
    override fun Content(modifier: Modifier) {
        Slider(
            value = value,
            onValueChange = {
                value = it
                onChange?.invoke(Math.round(it).toLong())
            },
            modifier = modifier,
            valueRange = min..maxOf(min, max),
        )
    }
}

class ProgressNode(private val spinner: Boolean) : Node() {
    /** 0..1 */
    var progress by mutableFloatStateOf(0f)

    @Composable
    override fun Content(modifier: Modifier) {
        if (spinner) CircularProgressIndicator(modifier) else LinearProgressIndicator({ progress }, modifier)
    }
}

class ImageNode : Node() {
    var bitmap by mutableStateOf<Bitmap?>(null)
    var description by mutableStateOf<String?>(null)

    @Composable
    override fun Content(modifier: Modifier) {
        val bitmap = bitmap ?: return Box(modifier)
        Image(
            bitmap.asImageBitmap(),
            description,
            modifier.semantics { description?.let { contentDescription = it } },
        )
    }

    companion object {
        @JvmStatic
        fun decode(bytes: ByteArray): Bitmap? = BitmapFactory.decodeByteArray(bytes, 0, bytes.size)
    }
}

/** Overlays its children; Rust keeps [children] in component order. */
class StackNode : Node() {
    val children = mutableStateListOf<Node>()

    /** Index into a 3x3 grid, row major: 0 = top start ... 4 = center ... 8 = bottom end. */
    var alignment by mutableIntStateOf(4)

    fun insertChild(index: Int, child: Node) {
        children.add(index, child)
    }

    fun removeChildAt(index: Int) {
        children.removeAt(index)
    }

    @Composable
    override fun Content(modifier: Modifier) {
        Box(modifier, contentAlignment = ALIGNMENTS[alignment]) {
            for (child in children) key(child) { child.Content(Modifier) }
        }
    }

    private companion object {
        val ALIGNMENTS = arrayOf(
            Alignment.TopStart, Alignment.TopCenter, Alignment.TopEnd,
            Alignment.CenterStart, Alignment.Center, Alignment.CenterEnd,
            Alignment.BottomStart, Alignment.BottomCenter, Alignment.BottomEnd,
        )
    }
}
