# Development Notes: Linux Playback and Cross-Course Data

These notes summarize the changes in this Linux-focused fork relative to
upstream Ckourse v1.2.0, along with verification status and follow-up work.

## Repository state

- Upstream base: `92cbe59` (`v1.2.0`, `main`).
- Working branch: `wip/appimage-media-framework`.
- Progress/sync baseline: `05079c8` (`Improve Linux packaging and sync shared video progress`), before the fork branding changes.
- The branch also retains the playback checkpoint `4c1a7bd`, the intermediate
  AppImage diagnosis checkpoint `67ac24c`, and the AppImage packaging fix
  `a7acadc`.
- The offline AppImage-runtime cache experiment was reverted and is not part of
  the current source tree.
- The app display name and npm project name are branded `Ckourse Linux`; the
  upstream icon artwork is retained, with a Linux badge added to the sidebar
  wordmark. The bundle identifier remains upstream-compatible.

## Changes from upstream

### Linux `stream://` playback

The application uses a custom GStreamer URI handler for streamed remote videos.
The Linux runtime setup now allows WebKit/GStreamer to use the `stream` URI
scheme and configures the custom plugin search path. The pinned Tauri GStreamer
plugin is patched from the `asset` scheme to `stream` and built by
`scripts/build-native-stream-plugin.sh`.

FFmpeg and FFprobe sidecars are staged for the current Rust target. They are
used by playback and media/subtitle handling. The known-working playback
checkpoint is:

- Commit: `4c1a7bd` — `Fix Linux GStreamer stream playback`
- Tag: `stream-playback-working-2026-10-13`

### Linux AppImage packaging

`scripts/build-appimage.sh` provides the post-processing used for the AppImage:

- Builds the Tauri Linux bundles and records the build log.
- Adds Ckourse's custom GStreamer plugin directory to the generated GStreamer
  AppRun hook.
- Removes the Wayland, XKB, and XCB libraries identified as conflicting with
  the host display stack.
- Repackages the modified AppDir using the cached linuxdeploy AppImage plugin.
- Allows the expected missing-updater-signing-key case only when Tauri reports
  that the bundles were generated.

The build command is `npm run appimage:build`. The native-plugin preparation
step fetches its pinned Git checkout, and AppImage tooling may download its
runtime; a fresh build therefore needs network access unless those dependencies
are already cached. The AppImage packaging fix is `a7acadc`.

### Remote course parsing and media handling

Changes across `src-tauri/src/parser.rs`, the remote-provider modules, and the
Google Drive/remote commands improve remote course discovery and streaming:

- Recognizes videos and subtitle files across remote folder layouts, including
  nested section and subtitle folders.
- Filters sample/trailer videos, identifies section-like folder names, and
  improves lesson sorting and title cleanup for numbered lectures.
- Avoids accepting empty remote folders as courses and supports flat remote
  course layouts.
- Supports remote video metadata and range-based streaming, including bounded
  open-ended range requests used when a media client seeks.
- Extracts embedded subtitle tracks to WebVTT with FFmpeg where applicable.
- Adds/updates remote browsing and Google authorization response handling.

Relevant implementation is in `src-tauri/src/parser.rs`,
`src-tauri/src/remote/`, `src-tauri/src/google.rs`,
`src-tauri/src/stream_cache.rs`, `src-tauri/src/video_protocol.rs`, and
`src-tauri/src/subtitle.rs`.

### Shared progress for duplicate videos

Lesson rows in different courses that have the same exact `video_path` share
video-level state:

- Newly imported lessons inherit the maximum existing completion value,
  positive duration, and resume position for that path.
- Completion toggles and resume-position/duration updates propagate to all
  matching lesson rows.
- Resetting a course clears completion for videos in that course across their
  duplicate entries, while clearing the selected course's last-watched markers.
- Course timestamps are refreshed for courses affected by shared progress.

This identity is exact path equality; files at different paths are not
considered duplicates solely because their contents match.

### Notes shared across duplicate videos

Notes now include the associated `video_path` in the Rust/TypeScript models.
Course note queries can discover notes associated with the same video in another
course, and the course detail UI maps those notes to the active lesson for
display. On course deletion, eligible notes are reassigned to a remaining
course/lesson with the same video path before cascading deletion.

The main implementation is in `src-tauri/src/db.rs`,
`src/pages/CourseDetail.tsx`, and `src/types/course.ts`.

## Known issues and unverified areas

- **Seeking can flicker.** Playback works, but the visual flicker during seek
  remains unresolved and needs a focused reproduction/investigation.
- **DEB and RPM are not functionally tested.** Tauri generated these bundles,
  but they have not been installed and exercised on a clean system. Do not
  treat generation as validation.
- **AppImage is the tested Linux package.** It was built and launched in the
  `ckourse24` Distrobox; streamed playback was reported working. A clean-host
  playback test is still useful for detecting undeclared host dependencies.
- Cross-course synchronization should continue to be tested for duplicate-course
  import, completion toggle, resume position, note visibility, note survival
  after deleting one duplicate course, and progress reset.
- Matching uses `video_path` equality. Remote URL normalization and different
  local paths to identical files are not unified by content identity.
- AppImage packaging depends on external tooling and may fail during Tauri's
  initial linuxdeploy pass; the post-processing script handles the observed
  generated-AppDir workflow. Updater artifacts are not produced without the
  private signing key.
- Before distributing independent releases, configure a fork-owned updater
  endpoint and signing key. The current updater endpoint still points to
  upstream Ckourse.

## Verification recorded

- `npm run build` passed in `ckourse24`.
- `cargo test` passed: 36 Rust tests.
- `npm run appimage:build` produced a launchable AppImage in `ckourse24`.
- The AppImage output is generated under
  `src-tauri/target/release/bundle/appimage/` and is not committed to Git.
- The dev app was launched with `npm run tauri dev`; this requires the pinned
  GStreamer plugin checkout used by the `pretauri` hook.

## Suggested follow-up

1. Reproduce and fix the seek flicker without regressing stream seeking.
2. Install and test the DEB and RPM packages in clean environments, or clearly
   document that only the AppImage is a supported/tested deliverable.
3. Run the cross-course progress/notes scenarios above against a disposable
   database.
4. Test the final AppImage on a clean Linux host and verify streamed playback
   without undeclared build-container dependencies.
5. If AppImage is the only required deliverable, consider an AppImage-only
   build mode; it was measured substantially faster, but the current build
   script still requests all Tauri bundle formats.
