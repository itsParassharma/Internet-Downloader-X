# IDX 0.7.0 public release preparation

The publisher supplied the Chrome Web Store PUBLIC key on 18 October 2026. Its SHA-256-derived extension ID is `bokfgodnpemfnmflphpcoccandblongm`, matching the existing store item. The unpacked manifest and embedded Rust public key were synchronized. No application logic, native UI geometry, download data, or private signing material was changed.

## Validation in this session

- `cargo fmt --all -- --check`: passed.
- `cargo test --locked`: 104 application tests passed; two Windows DPAPI tests failed. The packager stopped on that failure.
- `cargo test --locked --bin idx-native-host`: 25 tests passed; its DPAPI test failed.
- A separate synthetic `CryptProtectData` probe also failed with Windows error 2 in this restricted execution environment. This establishes that the problem is reproducible outside IDX code; it does not replace a normal-user acceptance run.
- `cargo build --release --locked --bins`: passed after the identity update; both MSVC executables were rebuilt.
- `python tools/package_release.py --skip-build`: produced the installer, portable ZIP and extension ZIP using those rebuilt executables. All 97 extension checks, the 18,979-reference integrity audit, column verification, extension-identity check and Windows import checks passed. `--skip-build` was used because compilation had already run separately; the failed regression results above remain failures.
- No encryption was removed, weakened or replaced, and no failing test was skipped to claim a passing regression run.

The earlier 132-test and real Chromium acceptance evidence in BUILD-VALIDATION.md remains historical evidence for the development extension identity. It must not be described as a newly executed Store-identity acceptance run. Full original visual equivalence and the function-by-function migration remain unfinished.

## GitHub Windows regression

The public source commit `d0e7a950d0ddae6d4d7cb4e9761efcbf0e258612` passed the full Windows CI workflow on 18 October 2026, including formatting, browser identity verification, pinned media helper verification, all 132 Rust tests (106 application and 26 native-host tests), the extension checks and release compilation. The three DPAPI tests that failed in the restricted local session passed on GitHub's Windows runner.

Evidence: [Windows checks run 39084105562](https://github.com/itsParassharma/IDX-Internet-Downloader-X/actions/runs/39084105562), job `117546957136`.

This verifies the published Store-identity source on a normal Windows CI runner. The installer attached to the release was built locally as described above; it was not replaced by the CI executables. CI does not establish manual desktop acceptance or complete original function equivalence.

## Publication access

Chrome rejected access to the developer console with "The extensions gallery cannot be scripted" and rejected a new-tab attempt with "Not allowed". Windows Computer Use also reported that its native pipe was unavailable, including after resetting the session and retrying following the user's plugin setup check. Store listing edits, upload, review submission and publication are therefore not verified by this preparation record. A current desktop-app screenshot also remains pending.

