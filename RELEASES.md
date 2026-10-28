# AmberLink automated releases

GitHub Actions validates and builds pushes to `main` and `beta_development`. A release is published only after both Linux and Windows builds, the regression suite, and the Linux LLVM native-compilation smoke test succeed.

## Version format

The published software version is:

`YYYY.RELEASE_SEQUENCE.PATCH`

- `YYYY` is calculated from the UTC year at the start of the release workflow.
- `RELEASE_SEQUENCE` identifies a release series within that year.
- `PATCH` identifies a maintenance revision within that series.
- `BUILD_NUMBER` is allocated by the release workflow's repository-wide GitHub Actions run number. It is shared by stable and beta pushes and is not reset at New Year. Failed release runs can leave gaps.

The sequence and patch are stored in [`.github/release-version.json`](.github/release-version.json). Change these deliberately when starting a new release series or maintenance revision; ordinary commits do not change the software version.

For example, with `release_sequence: 1` and `patch: 0` in 2026:

- Stable release title: `AmberLink 2026.1.0 (Build 143)`
- Stable Git tag: `v2026.1.0+build.143`
- Beta release title: `AmberLink 2026.1.0-beta.144`
- Beta Git tag: `v2026.1.0-beta.144`

Stable releases are created from `main` and are not marked as prereleases. Builds from `beta_development` are marked as GitHub prereleases. The tag includes the build number so every push can have a distinct release even when the software version remains the same.

## Included automation

- Pull-request CI runs on Linux and Windows.
- CI runs Rust tests, Clippy, builds the Rust compiler and C++ AVM, runs the Amberlink regression suite, and smoke-tests LLVM native compilation on Linux.
- Successful pushes to `main` and `beta_development` publish downloadable Linux x64 and Windows x64 packages.
- The release workflow creates or updates the matching GitHub Release and generates release notes.
- Issue and pull-request automation creates and applies managed labels based on title, changed paths, and PR size.
- Dependabot checks Rust dependencies and GitHub Actions weekly.

Rust formatting is currently an advisory CI check because existing source files do not yet pass `cargo fmt --check`; it can become blocking after the existing formatting baseline is normalized.
