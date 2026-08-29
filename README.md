# bibimapy

`bibimapy` is a native [Tauri 2](https://v2.tauri.app/) desktop shell for
[marimo](https://marimo.io/). It bundles the Rust-based `uv` executable and uses
it to install Python 3.12, create an isolated virtual environment, and download
marimo on first launch.

## What is included

- Tauri 2 native window on Windows, macOS, and Linux.
- marimo served only on `127.0.0.1` and embedded in the Tauri window.
- A bundled `uv` sidecar; no system Python is required.
- Default Python 3.12 environment in `~/.bibimapy/venv`.
- TOML settings at `~/.bibimapy/config.toml`.
- Configurable Python package mirror, with Aliyun selected by default on a
  Chinese system locale.
- `rust-i18n` translations for English, Simplified Chinese, Japanese, Korean,
  Russian, French, Spanish, Portuguese, Italian, German, and Latin.
- Tagged GitHub releases for Windows x64, macOS Apple Silicon/Intel, and Linux
  x64.
- Windows x64 installer and portable ZIP release variants. The portable ZIP
  contains `bibimapy.exe`, its `uv.exe` sidecar, and requires no installation.

## Runtime layout

```text
~/.bibimapy/
├── config.toml        # User settings
├── python/            # Python builds installed by uv
├── venv/              # Isolated marimo environment
├── environment.toml   # Managed-environment fingerprint
├── cache/             # uv download cache
├── notebooks/home.py  # Starter notebook (kept across upgrades)
└── logs/marimo.log    # Local server output
```

The default configuration is equivalent to:

```toml
language = "system"
python_version = "3.12"
pip_index_url = "https://pypi.org/simple"
marimo_package = "marimo"
marimo_port = 2718
startup_timeout_seconds = 600
```

On the first launch, bibimapy runs the conceptual sequence below through its
bundled sidecar:

```shell
uv python install 3.12
uv venv ~/.bibimapy/venv --python 3.12
uv pip install --python ~/.bibimapy/venv/bin/python marimo
```

`UV_PYTHON_INSTALL_DIR`, `UV_CACHE_DIR`, and `UV_DEFAULT_INDEX` keep the
environment isolated and apply the selected mirror. On Windows, the venv
interpreter is `~/.bibimapy/venv/Scripts/python.exe`.

## Develop locally

Prerequisites are the normal [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/),
Node.js 22+, a current Rust toolchain, and `uv` available on `PATH`.

```shell
npm install
npm run tauri:dev
```

`npm run tauri:dev` copies the installed `uv` executable to the target-specific
sidecar name expected by Tauri, then starts Vite and the native application.
Set `BIBIMAPY_UV` to an explicit `uv` path when it is not on `PATH`.

Useful checks:

```shell
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

## Releases

Push a semantic version tag to build and publish native installers:

```shell
git tag v0.1.0
git push origin v0.1.0
```

The release workflow pins `uv` 0.12.0, prepares the correct sidecar for every
target, builds native packages, uploads workflow artifacts, and attaches them
to the GitHub release. Windows releases include both an installer and
`bibimapy_<version>_windows_x64_portable.zip`. The two executables inside the
portable archive must remain in the same directory so bibimapy can find its uv
sidecar. Code signing/notarization can be added later through the standard
Tauri signing environment variables and repository secrets.

To package the Windows portable build locally after `tauri build`:

```powershell
./scripts/package-portable.ps1
```

## Security model

marimo binds to loopback only and starts with `--no-token`; it is not exposed to
the LAN. Tauri's content security policy permits framing and WebSocket access
only to `127.0.0.1`. The process is terminated when the desktop application
exits.

## License

Apache-2.0. See [LICENSE](LICENSE).
