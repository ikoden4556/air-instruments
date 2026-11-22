package com.airinstruments

/** On-screen Undo, Clear and Key (same as web/src/controls.ts). */
object Controls {
    val KEY_NAMES = listOf("C", "C♯", "D", "E♭", "E", "F", "F♯", "G", "A♭", "A", "B♭", "B")

    sealed interface Control {
        data object Undo : Control
        data object Clear : Control
        data class Key(val root: Int) : Control
    }

    /** What the controls change in the Engine. */
    interface Controlled {
        fun undo()
        fun clear()
        fun setKey(root: Int)
    }

    fun apply(control: Control, engine: Controlled) = when (control) {
        Control.Undo -> engine.undo()
        Control.Clear -> engine.clear()
        is Control.Key -> engine.setKey(Math.floorMod(control.root, 12))
    }
}
