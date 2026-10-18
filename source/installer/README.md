# IDX Windows installer

Build with `python tools/package_release.py` from the source folder. Inno Setup 6 and the compiled x64 release binaries are required. Setup installs for the current user, preserves the download library, and can create an optional desktop shortcut.

The installer has no browser ID page or browser-integration task. The app registers the bundled extension automatically when opened. First launch offers extension-install instructions; Options > General > Extension setup also provides Reconnect browser. Setup can be run silently with `/VERYSILENT /SUPPRESSMSGBOXES /NORESTART`.

`AppIdentity` and the compiler output path can be overridden for isolated developer acceptance. Tests must use a separate AppId, `/NOICONS` and a separate data directory/instance port, and restore native-host registry values afterwards.

The uninstaller removes only native-host registry entries pointing at its own installation. It retains library and downloaded files. For the eventual store release, synchronize the draft's public key with `tools/sync_extension_identity.py`, then rebuild the desktop and extension together; end users do not supply an ID.
