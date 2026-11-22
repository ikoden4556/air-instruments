# air-instruments

An air string instrument played with bare hands in front of a phone camera. Rust core, thin shells: a TypeScript/Vite web app (the iPhone path) and a native Android app in Kotlin.

Read before working:
- `BLUEPRINT.md`: decisions, playing rules S1–S8, latency targets, tech stack, phases, deferred latency fixes.
- `GLOSSARY.md`: domain terms. Use them exactly (Pluck, not "hit"; Player, not "user").
- `docs/adr/`: why things are the way they are. ADR 0004 holds the measured latency targets.

## How work is tracked
- One GitHub issue per piece of work (repo `ikoden4556/air-instruments`). The issue is the spec.
- `/implement <n>` starts or resumes issue n. `/handoff` saves progress to the issue before a `/clear`.
- Branch per issue: `issue-<n>-<slug>`. PR body says `Closes #<n>`.
- The phase 2 latency spike lives on branch `spike/latency` (results in `spike/RESULTS.md` there). Never merge it into main.

## Working rules
- One step at a time; explain plainly. The user often follows from a phone over Remote Control, so replies must read well as plain text, and questions must be in the message itself.
- Smallest change that fully solves the task (ponytail lite). Name skipped or unverified things at the end.
- Core work is test-first (`tdd` skill), `cargo test` in `core/`.
- Commit as the GitHub noreply address; never put the user's email anywhere.
- Don't change system or security settings (firewall, hosts, env vars); ask the user.

## Toolchain (Windows, everything on D:)
Set these in each PowerShell call; they are not on the default PATH:
```powershell
$env:RUSTUP_HOME='D:\dev\rust\rustup'; $env:CARGO_HOME='D:\dev\rust\cargo'; $env:Path="D:\dev\rust\cargo\bin;D:\dev\nodejs;$env:Path"
$env:JAVA_HOME='D:\dev\AndroidStudio\jbr'; $env:GRADLE_USER_HOME='D:\dev\gradle'; $env:ANDROID_HOME='D:\dev\Android\Sdk'
```
- Rust → Android `.so`: `cargo ndk -t arm64-v8a -P 29 -o <app>\src\main\jniLibs build --release`
- Android build: `.\gradlew.bat assembleDebug` (Gradle 9.9.2, AGP 9.5.2, compileSdk 37, minSdk 29)
- adb: `D:\dev\Android\Sdk\platform-tools\adb.exe`
- No Python on this machine.

## Test phone
- Oppo Find X9 Pro over USB with debugging on. It disconnects sometimes; ask the user to replug.
- Web dev server: phones can't reach the PC over Wi-Fi, so use `adb reverse tcp:5173 tcp:5173` and open `https://localhost:5173`.
- Read the phone's screen yourself: `adb exec-out screencap -p > file.png`, then view the PNG.
- Front camera reaches 60 fps only via CameraX `Preview.Builder.setTargetFrameRate`, with the preview bound.
