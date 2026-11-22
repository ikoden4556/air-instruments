package com.airinstruments

/** The last `capacity` values of a measurement, for an average, p95 and minimum. */
class Rolling(private val capacity: Int = 60) {
    private val values = ArrayDeque<Double>()

    @Synchronized fun add(v: Double) {
        values.addLast(v)
        if (values.size > capacity) values.removeFirst()
    }

    @Synchronized fun summary(): String {
        if (values.isEmpty()) return "n/a"
        val s = values.sorted()
        val p95 = s[minOf(s.size - 1, (s.size * 0.95).toInt())]
        return "avg %.0f  p95 %.0f  min %.0f ms".format(s.average(), p95, s.first())
    }
}

/**
 * Felt latency per Pluck: when its sound reached the speaker minus when the Plucker crossed
 * (for a predicted Pluck, when it was predicted to cross). The Engine thread reports each
 * Pluck; the readout polls the audio stream for the last Pluck it started.
 */
class FeltLatency {
    val all = Rolling()
    val predicted = Rolling()
    val real = Rolling()
    private val wasPredicted = LinkedHashMap<Long, Boolean>()
    private var lastCrossing = Long.MIN_VALUE

    @Synchronized fun onPluck(crossingNs: Long, predicted: Boolean) {
        wasPredicted[crossingNs] = predicted
        while (wasPredicted.size > 64) wasPredicted.remove(wasPredicted.keys.first())
    }

    /** Records the Pluck the synth started last, once; returns its felt latency in ms if new. */
    @Synchronized fun onSpeaker(crossingNs: Long, presentedNs: Long): Double? {
        if (crossingNs == lastCrossing) return null
        lastCrossing = crossingNs
        val ms = (presentedNs - crossingNs) / 1e6
        all.add(ms)
        when (wasPredicted[crossingNs]) {
            true -> predicted.add(ms)
            false -> real.add(ms)
            null -> {}
        }
        return ms
    }
}
