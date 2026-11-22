package com.airinstruments

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class ShortExposureTest {
    private val ms = 1_000_000L

    @Test
    fun a_long_auto_exposure_becomes_the_cap_with_iso_raised_to_keep_the_brightness() {
        val s = ShortExposure.from(aeExposureNs = 50 * ms, aeIso = 400, maxIso = 6400)!!
        assertEquals(ShortExposure.CAP_NS, s.exposureNs)
        assertEquals(400 * 50 * ms / ShortExposure.CAP_NS, s.iso.toLong())
    }

    @Test
    fun in_dim_light_iso_stops_at_the_sensor_maximum_and_the_picture_gets_darker() {
        val s = ShortExposure.from(aeExposureNs = 60 * ms, aeIso = 6000, maxIso = 6400)!!
        assertEquals(ShortExposure.CAP_NS, s.exposureNs)
        assertEquals(6400, s.iso)
    }

    @Test
    fun an_auto_exposure_already_short_enough_is_left_alone() {
        assertNull(ShortExposure.from(aeExposureNs = 10 * ms, aeIso = 200, maxIso = 6400))
    }

    @Test
    fun the_cap_allows_60_fps() {
        assertEquals(true, ShortExposure.CAP_NS <= ShortExposure.FRAME_NS)
        assertEquals(16_666_666L, ShortExposure.FRAME_NS)
    }
}
