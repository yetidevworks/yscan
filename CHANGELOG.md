## Changelog

### 0.1.1

- Relicensed from MIT to Apache 2.0. The published crate and the Homebrew formula both still claimed MIT.
- Release workflow now verifies the tag against the crate version before anything publishes, so a mismatched tag can no longer burn a version number on crates.io.
- Release workflow triggers on bare version tags (`0.1.1`) instead of `v`-prefixed ones.
- Added `homepage` to the package metadata.
- Added a screenshot to the README, plus formatting fixes.

### v0.1.0

- Initial release
