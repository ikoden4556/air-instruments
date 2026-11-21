# air-instruments

A string instrument played in the air in front of a phone camera: the Player stretches Strings between their hands and plucks them with a fingertip.

## Building strings

**Player**:
The person in front of the camera playing the instrument.
_Avoid_: User, musician

**Pinch**:
A Hand state in which the thumb tip and index fingertip are held together.
_Avoid_: Grab, grip, hold

**Anchor**:
The point between thumb and index fingertip of a pinching Hand, where it holds one end of a Stretch.
_Avoid_: Handle, grip point

**Stretch**:
A not-yet-playable string held between two Anchors, one on each Hand, existing only while both Hands Pinch. It never Plucks anything.
_Avoid_: Preview, draft, pending string

**Freeze**:
The moment a Stretch becomes a String fixed in the air, when the first of its two Hands stops Pinching.
_Avoid_: Lock, place, drop, release

**Discard**:
A Stretch ending without becoming a String, because it is too short or a Hand stayed out of sight.
_Avoid_: Cancel, delete

**String**:
A straight segment frozen in the air that can be Plucked.
_Avoid_: Line, wire, chord

## Playing

**Plucker**:
The index fingertip of a Hand that is not Pinching.
_Avoid_: Pick, cursor, striker

**Pluck**:
A Plucker crossing a String between one camera frame and the next. A String Plucks at most once per crossing, however many Pluckers cross it.
_Avoid_: Hit, touch, trigger

**Strum**:
One swipe of a Plucker that Plucks several Strings in order.
_Avoid_: Sweep

**Velocity**:
How hard a Pluck is, taken from the Plucker's speed as it crosses the String.
_Avoid_: Force, strength

## Pitch

**Key**:
The pentatonic scale every note snaps to.
_Avoid_: Tuning, mode

**Base note**:
The note a String plays at its first Fret, set by the String's length: shorter is higher.
_Avoid_: Root, open note

**Fret**:
One of the equal sections a String is divided into, marked by ticks; each Fret further along raises the note one step in the Key.
_Avoid_: Tick, zone, segment

## Sound

**Voice**:
One sounding, fading instance of a Pluck.
_Avoid_: Note, sound, channel

**Instrument**:
The sound character all Voices share (v1: warm steel-string guitar).
_Avoid_: Preset, patch, timbre

**Motion-to-sound latency**:
The time from a Plucker physically crossing a String to the Player hearing the Pluck.
_Avoid_: Lag, delay, latency (unqualified)
