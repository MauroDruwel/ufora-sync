"""Cross-platform startup manager for Ufora Sync (LaunchAgent, Registry, XDG)."""

from __future__ import annotations

import contextlib
import os
import shutil
import subprocess
import sys
from pathlib import Path


def _get_ufora_sync_exe() -> str:
    """Return the absolute path to the ufora-sync executable."""
    exe = shutil.which("ufora-sync")
    if exe:
        return str(Path(exe).resolve())
    fallback = Path(sys.executable).parent / "ufora-sync"
    if fallback.exists():
        return str(fallback.resolve())
    return sys.executable


# ---------------------------------------------------------------------------
# macOS LaunchAgent (~/Library/LaunchAgents/com.maurodruwel.ufora-sync.plist)
# ---------------------------------------------------------------------------

MACOS_LABEL = "com.maurodruwel.ufora-sync"


def _macos_plist_path() -> Path:
    return Path.home() / "Library" / "LaunchAgents" / f"{MACOS_LABEL}.plist"


def _macos_generate_plist(exe_path: str) -> str:
    return f"""<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{MACOS_LABEL}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{exe_path}</string>
        <string>tray</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>ProcessType</key>
    <string>Interactive</string>
</dict>
</plist>
"""


def _macos_is_enabled() -> bool:
    return _macos_plist_path().exists()


def _macos_enable() -> tuple[bool, str]:
    plist_file = _macos_plist_path()
    plist_file.parent.mkdir(parents=True, exist_ok=True)
    exe_path = _get_ufora_sync_exe()

    try:
        if plist_file.exists():
            subprocess.run(
                ["launchctl", "unload", str(plist_file)],
                check=False,
                capture_output=True,
            )
        plist_file.write_text(_macos_generate_plist(exe_path), encoding="utf-8")
        subprocess.run(["launchctl", "load", str(plist_file)], check=False, capture_output=True)
        return True, f"Auto-start enabled via LaunchAgent ({plist_file})"
    except Exception as exc:
        return False, f"Failed enabling LaunchAgent: {exc}"


def _macos_disable() -> tuple[bool, str]:
    plist_file = _macos_plist_path()
    if not plist_file.exists():
        return True, "Auto-start is already disabled"

    try:
        subprocess.run(["launchctl", "unload", str(plist_file)], check=False, capture_output=True)
        plist_file.unlink(missing_ok=True)
        return True, "Auto-start disabled"
    except Exception as exc:
        return False, f"Failed disabling LaunchAgent: {exc}"


# ---------------------------------------------------------------------------
# Linux XDG Autostart (~/.config/autostart/ufora-sync.desktop)
# ---------------------------------------------------------------------------


def _linux_desktop_path() -> Path:
    config_home = os.environ.get("XDG_CONFIG_HOME", str(Path.home() / ".config"))
    return Path(config_home) / "autostart" / "ufora-sync.desktop"


def _linux_is_enabled() -> bool:
    return _linux_desktop_path().exists()


def _linux_enable() -> tuple[bool, str]:
    desktop_file = _linux_desktop_path()
    desktop_file.parent.mkdir(parents=True, exist_ok=True)
    exe_path = _get_ufora_sync_exe()

    content = f"""[Desktop Entry]
Type=Application
Name=Ufora Sync
Comment=UGent Ufora background sync service
Exec={exe_path} tray
Hidden=false
NoDisplay=false
X-GNOME-Autostart-enabled=true
"""
    try:
        desktop_file.write_text(content, encoding="utf-8")
        return True, f"Auto-start enabled ({desktop_file})"
    except Exception as exc:
        return False, f"Failed enabling autostart: {exc}"


def _linux_disable() -> tuple[bool, str]:
    desktop_file = _linux_desktop_path()
    if not desktop_file.exists():
        return True, "Auto-start is already disabled"
    try:
        desktop_file.unlink(missing_ok=True)
        return True, "Auto-start disabled"
    except Exception as exc:
        return False, f"Failed disabling autostart: {exc}"


# ---------------------------------------------------------------------------
# Windows Registry (HKCU\Software\Microsoft\Windows\CurrentVersion\Run)
# ---------------------------------------------------------------------------


def _windows_is_enabled() -> bool:
    if sys.platform != "win32":
        return False
    try:
        import winreg

        with winreg.OpenKey(
            winreg.HKEY_CURRENT_USER,
            r"Software\Microsoft\Windows\CurrentVersion\Run",
            0,
            winreg.KEY_READ,
        ) as key:
            winreg.QueryValueEx(key, "UforaSync")
            return True
    except OSError:
        return False


def _windows_enable() -> tuple[bool, str]:
    if sys.platform != "win32":
        return False, "Not running on Windows"
    try:
        import winreg

        exe_path = _get_ufora_sync_exe()
        cmd = f'"{exe_path}" tray'
        with winreg.OpenKey(
            winreg.HKEY_CURRENT_USER,
            r"Software\Microsoft\Windows\CurrentVersion\Run",
            0,
            winreg.KEY_SET_VALUE,
        ) as key:
            winreg.SetValueEx(key, "UforaSync", 0, winreg.REG_SZ, cmd)
        return True, "Auto-start enabled in Windows registry"
    except Exception as exc:
        return False, f"Failed enabling Windows auto-start: {exc}"


def _windows_disable() -> tuple[bool, str]:
    if sys.platform != "win32":
        return True, "Not running on Windows"
    try:
        import winreg

        with (
            winreg.OpenKey(
                winreg.HKEY_CURRENT_USER,
                r"Software\Microsoft\Windows\CurrentVersion\Run",
                0,
                winreg.KEY_SET_VALUE,
            ) as key,
            contextlib.suppress(FileNotFoundError),
        ):
            winreg.DeleteValue(key, "UforaSync")
        return True, "Auto-start disabled"
    except Exception as exc:
        return False, f"Failed disabling Windows auto-start: {exc}"


# ---------------------------------------------------------------------------
# Public Unified Interface
# ---------------------------------------------------------------------------


def is_autostart_enabled() -> bool:
    """Check if Ufora Sync is configured to launch automatically on login."""
    if sys.platform == "darwin":
        return _macos_is_enabled()
    if sys.platform == "win32":
        return _windows_is_enabled()
    return _linux_is_enabled()


def enable_autostart() -> tuple[bool, str]:
    """Configure Ufora Sync to launch automatically on login."""
    if sys.platform == "darwin":
        return _macos_enable()
    if sys.platform == "win32":
        return _windows_enable()
    return _linux_enable()


def disable_autostart() -> tuple[bool, str]:
    """Remove Ufora Sync from automatic login startup."""
    if sys.platform == "darwin":
        return _macos_disable()
    if sys.platform == "win32":
        return _windows_disable()
    return _linux_disable()
