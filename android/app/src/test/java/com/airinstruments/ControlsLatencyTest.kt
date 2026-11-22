package com.airinstruments

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class ControlsLatencyTest {
    private class FakeEngine : Controls.Controlled {
        val calls = mutableListOf<String>()
        override fun undo() { calls += "undo" }
        override fun clear() { calls += "clear" }
        override fun setKey(root: Int) { calls += "key $root" }
    }

    @Test
    fun undo_clear_and_key_reach_the_engine_and_key_roots_wrap() {
        val e = FakeEngine()
        Controls.apply(Controls.Control.Undo, e)
        Controls.apply(Controls.Control.Clear, e)
        Controls.apply(Controls.Control.Key(7), e)
        Controls.apply(Controls.Control.Key(14), e)
        Controls.apply(Controls.Control.Key(-1), e)
        assertEquals(listOf("undo", "clear", "key 7", "key 2", "key 11"), e.calls)
        assertEquals(12, Controls.KEY_NAMES.size)
        assertEquals("A", Controls.KEY_NAMES[9])
    }

    @Test
    fun rolling_stats_summarise_the_last_values() {
        val r = Rolling(capacity = 3)
        assertEquals("n/a", r.summary())
        for (v in listOf(1000.0, 80.0, 90.0, 100.0)) r.add(v) // 1000 falls out
        assertEquals("avg 90  p95 100  min 80 ms", r.summary())
    }

    @Test
    fun felt_latency_is_speaker_time_minus_crossing_time_split_by_prediction() {
        val f = FeltLatency()
        f.onPluck(1_000_000_000L, predicted = true)
        assertEquals(86.0, f.onSpeaker(1_000_000_000L, 1_086_000_000L)!!, 1e-9)
        assertNull(f.onSpeaker(1_000_000_000L, 1_086_000_000L)) // same Pluck read again
        f.onPluck(2_000_000_000L, predicted = false)
        assertEquals(133.0, f.onSpeaker(2_000_000_000L, 2_133_000_000L)!!, 1e-9)
        assertEquals("avg 86  p95 86  min 86 ms", f.predicted.summary())
        assertEquals("avg 133  p95 133  min 133 ms", f.real.summary())
        assertEquals("avg 110  p95 133  min 86 ms", f.all.summary())
    }
}
