# The web app is the iOS path; no native iOS app

There is no Mac available, and building native iOS apps requires one (or a cloud Mac plus a $99/year Apple Developer account). iPhones run the browser web app instead, installed as a PWA from GitHub Pages. Native Android exists alongside it as the lowest-latency version. A native iOS app can be added later on the same Rust core (ADR 0001) if a Mac becomes available.

## Consequences

- On iPhone, Motion-to-sound latency is limited by Safari (camera usually 30 fps, plus Web Audio output delay): target under 100 ms rather than under 60 ms.
- The repo is public, because GitHub Pages on a free account only serves public repos.
