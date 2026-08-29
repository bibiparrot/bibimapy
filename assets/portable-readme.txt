bibimapy portable for Windows x64
==================================

Keep bibimapy.exe and uv.exe in the same directory, then run bibimapy.exe.
No installer or system Python is required. On first launch, uv downloads the
configured Python 3.12 runtime, marimo, and its Python dependencies into:

    %USERPROFILE%\.bibimapy

Microsoft Edge WebView2 Runtime is required. It is included with current
Windows 10 and Windows 11 installations and can also be installed from
Microsoft if it is missing.

Configuration:

    %USERPROFILE%\.bibimapy\config.toml

The local marimo service listens only on 127.0.0.1 and stops when bibimapy
exits.
