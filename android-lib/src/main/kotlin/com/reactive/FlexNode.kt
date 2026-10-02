package com.reactive

import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.layout.IntrinsicMeasurable
import androidx.compose.ui.layout.IntrinsicMeasureScope
import androidx.compose.ui.layout.Layout
import androidx.compose.ui.layout.LayoutIdParentData
import androidx.compose.ui.layout.Measurable
import androidx.compose.ui.layout.MeasurePolicy
import androidx.compose.ui.layout.MeasureResult
import androidx.compose.ui.layout.MeasureScope
import androidx.compose.ui.layout.layoutId
import androidx.compose.ui.unit.Constraints
import kotlin.math.roundToInt

/**
 * A flex container laid out by the Rust flex engine inside the real Compose layout pass.
 *
 * Rust keeps [children] in the same order as its layout tree. During a measure pass Rust
 * sizes leaves through [intrinsic] (which Compose allows any number of times), then Kotlin
 * measures every child exactly once at the size Rust chose and places it.
 *
 * All values crossing into Rust are in dp.
 */
class FlexNode : Node() {
    val children = mutableStateListOf<Node>()

    /** Address of the Rust layout tree; 0 once the owning component is disposed. */
    var handle = 0L

    /** Bumped by Rust when layout inputs it tracks (props, modifiers) change. */
    private var layoutVersion by mutableIntStateOf(0)

    private var currentMeasurables: Map<Any?, IntrinsicMeasurable> = emptyMap()
    private var currentDensity = 1f

    fun insertChild(index: Int, child: Node) {
        children.add(index, child)
    }

    fun removeChildAt(index: Int) {
        children.removeAt(index)
    }

    fun invalidate() {
        layoutVersion++
    }

    /**
     * Called by Rust while [nativeMeasure] runs. [kind]: 0 = min width, 1 = max width,
     * 2 = min height, 3 = max height. [cross] is the other axis in dp, or negative if unbounded.
     */
    @Suppress("unused")
    fun intrinsic(child: Node, kind: Int, cross: Float): Float {
        val m = currentMeasurables[child] ?: return 0f
        val c = if (cross < 0) Constraints.Infinity else (cross * currentDensity).roundToInt()
        val px = when (kind) {
            0 -> m.minIntrinsicWidth(c)
            1 -> m.maxIntrinsicWidth(c)
            2 -> m.minIntrinsicHeight(c)
            else -> m.maxIntrinsicHeight(c)
        }
        return px / currentDensity
    }

    @Composable
    override fun Content(modifier: Modifier) {
        Layout(
            content = { for (child in children) key(child) { child.Content(Modifier.layoutId(child)) } },
            modifier = modifier,
            measurePolicy = policy,
        )
    }

    /**
     * Runs Rust layout. Known sizes are NaN when free; available sizes are dp, or [MIN_CONTENT]
     * / [MAX_CONTENT]. Returns `[width, height]`, followed by `x, y, width, height` for each
     * child when [layout] is true.
     */
    private fun runLayout(
        density: Float,
        measurables: List<IntrinsicMeasurable>,
        knownWidth: Float,
        knownHeight: Float,
        availableWidth: Float,
        availableHeight: Float,
        layout: Boolean,
    ): FloatArray? {
        layoutVersion // Read so Compose re-measures when Rust invalidates.
        if (handle == 0L) return null
        val saved = currentMeasurables to currentDensity
        currentMeasurables = measurables.associateBy { (it.parentData as? LayoutIdParentData)?.layoutId }
        currentDensity = density
        try {
            return nativeMeasure(handle, knownWidth, knownHeight, availableWidth, availableHeight, layout)
        } finally {
            currentMeasurables = saved.first
            currentDensity = saved.second
        }
    }

    private fun IntrinsicMeasureScope.intrinsic(
        measurables: List<IntrinsicMeasurable>,
        width: Boolean,
        min: Boolean,
        cross: Int,
    ): Int {
        val crossDp = if (cross == Constraints.Infinity) Float.NaN else cross / density
        val space = if (min) MIN_CONTENT else MAX_CONTENT
        val out = if (width) {
            runLayout(density, measurables, Float.NaN, crossDp, space, if (crossDp.isNaN()) MAX_CONTENT else crossDp, false)
        } else {
            runLayout(density, measurables, crossDp, Float.NaN, if (crossDp.isNaN()) MAX_CONTENT else crossDp, space, false)
        } ?: return 0
        return ((if (width) out[0] else out[1]) * density).roundToInt()
    }

    private val policy = object : MeasurePolicy {
        override fun MeasureScope.measure(measurables: List<Measurable>, constraints: Constraints): MeasureResult {
            fun known(min: Int, max: Int) = if (min == max) max / density else Float.NaN
            fun available(max: Int) = if (max == Constraints.Infinity) MAX_CONTENT else max / density

            val out = runLayout(
                density,
                measurables,
                known(constraints.minWidth, constraints.maxWidth),
                known(constraints.minHeight, constraints.maxHeight),
                available(constraints.maxWidth),
                available(constraints.maxHeight),
                true,
            ) ?: return layout(constraints.minWidth, constraints.minHeight) {}

            fun px(dp: Float) = (dp * density).roundToInt().coerceAtLeast(0)
            val byNode = measurables.associateBy { it.layoutId }
            val placed = children.mapIndexedNotNull { i, child ->
                val base = 2 + i * 4
                if (base + 3 >= out.size) return@mapIndexedNotNull null
                val m = byNode[child] ?: return@mapIndexedNotNull null
                Triple(m.measure(Constraints.fixed(px(out[base + 2]), px(out[base + 3]))), px(out[base]), px(out[base + 1]))
            }

            return layout(
                px(out[0]).coerceIn(constraints.minWidth, constraints.maxWidth),
                px(out[1]).coerceIn(constraints.minHeight, constraints.maxHeight),
            ) {
                for ((placeable, x, y) in placed) placeable.place(x, y)
            }
        }

        override fun IntrinsicMeasureScope.minIntrinsicWidth(measurables: List<IntrinsicMeasurable>, height: Int) =
            intrinsic(measurables, width = true, min = true, height)

        override fun IntrinsicMeasureScope.maxIntrinsicWidth(measurables: List<IntrinsicMeasurable>, height: Int) =
            intrinsic(measurables, width = true, min = false, height)

        override fun IntrinsicMeasureScope.minIntrinsicHeight(measurables: List<IntrinsicMeasurable>, width: Int) =
            intrinsic(measurables, width = false, min = true, width)

        override fun IntrinsicMeasureScope.maxIntrinsicHeight(measurables: List<IntrinsicMeasurable>, width: Int) =
            intrinsic(measurables, width = false, min = false, width)
    }

    private external fun nativeMeasure(
        handle: Long,
        knownWidth: Float,
        knownHeight: Float,
        availableWidth: Float,
        availableHeight: Float,
        layout: Boolean,
    ): FloatArray

    private companion object {
        const val MIN_CONTENT = -1f
        const val MAX_CONTENT = -2f
    }
}
