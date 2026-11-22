package com.airinstruments

import android.content.Context
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.view.Gravity
import android.view.View
import android.widget.Button
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.PopupMenu
import android.widget.TextView

/** The Undo, Clear, Key and latency-readout buttons along the top edge, right-aligned. */
fun controlBar(context: Context, engine: Controls.Controlled, onToggleReadout: () -> Unit): View {
    val dp = context.resources.displayMetrics.density
    fun button(label: String, onClick: (Button) -> Unit) = Button(context).apply {
        text = label
        isAllCaps = false
        textSize = 18f
        setTextColor(Color.WHITE)
        minWidth = (80 * dp).toInt()
        minHeight = (52 * dp).toInt()
        background = GradientDrawable().apply {
            cornerRadius = 26 * dp
            setColor(Color.argb(140, 0, 40, 40))
            setStroke((1 * dp).toInt(), Color.argb(150, 140, 255, 255))
        }
        setOnClickListener { onClick(this) }
    }
    return LinearLayout(context).apply {
        orientation = LinearLayout.HORIZONTAL
        val gap = (10 * dp).toInt()
        fun add(v: View) = addView(v, LinearLayout.LayoutParams(-2, -2).apply { marginStart = gap })
        add(button("ms") { onToggleReadout() })
        add(button("Undo") { Controls.apply(Controls.Control.Undo, engine) })
        add(button("Clear") { Controls.apply(Controls.Control.Clear, engine) })
        add(button("Key C") { b ->
            PopupMenu(context, b).apply {
                Controls.KEY_NAMES.forEachIndexed { root, name -> menu.add(0, root, root, "Key $name") }
                setOnMenuItemClickListener {
                    Controls.apply(Controls.Control.Key(it.itemId), engine)
                    b.text = it.title
                    true
                }
            }.show()
        })
        layoutParams = FrameLayout.LayoutParams(-2, -2, Gravity.TOP or Gravity.END).apply {
            setMargins(gap, gap, (12 * dp).toInt(), gap)
        }
    }
}

/** The latency readout, top-left; hidden until the "ms" button shows it. */
fun readoutView(context: Context): TextView = TextView(context).apply {
    typeface = Typeface.MONOSPACE
    textSize = 12f
    setTextColor(Color.WHITE)
    setBackgroundColor(Color.argb(153, 0, 0, 0))
    setPadding(16, 12, 16, 12)
    visibility = View.GONE
    layoutParams = FrameLayout.LayoutParams(-2, -2, Gravity.TOP or Gravity.START)
}
