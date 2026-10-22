"""Unit tests for the cross-platform autostart management module."""

from __future__ import annotations

import sys
from pathlib import Path

from ufora_sync.autostart import (
    _linux_disable,
    _linux_enable,
    _linux_is_enabled,
    _macos_disable,
    _macos_enable,
    _macos_generate_plist,
    _macos_is_enabled,
    disable_autostart,
    enable_autostart,
    is_autostart_enabled,
)


def test_macos_generate_plist() -> None:
    plist = _macos_generate_plist("/usr/local/bin/ufora-sync")
    assert "<string>com.maurodruwel.ufora-sync</string>" in plist
    assert "<string>/usr/local/bin/ufora-sync</string>" in plist
    assert "<string>tray</string>" in plist
    assert "<key>RunAtLoad</key>" in plist


def test_macos_autostart_lifecycle(monkeypatch, tmp_path: Path) -> None:
    plist_file = tmp_path / "com.maurodruwel.ufora-sync.plist"
    monkeypatch.setattr("ufora_sync.autostart._macos_plist_path", lambda: plist_file)
    monkeypatch.setattr("ufora_sync.autostart.subprocess.run", lambda *args, **kwargs: None)

    # Initially disabled
    assert _macos_is_enabled() is False

    # Enable
    ok, msg = _macos_enable()
    assert ok is True
    assert plist_file.exists()
    assert _macos_is_enabled() is True

    # Disable
    ok, msg = _macos_disable()
    assert ok is True
    assert not plist_file.exists()
    assert _macos_is_enabled() is False


def test_linux_autostart_lifecycle(monkeypatch, tmp_path: Path) -> None:
    desktop_file = tmp_path / "autostart" / "ufora-sync.desktop"
    monkeypatch.setattr("ufora_sync.autostart._linux_desktop_path", lambda: desktop_file)

    assert _linux_is_enabled() is False

    ok, msg = _linux_enable()
    assert ok is True
    assert desktop_file.exists()
    assert "Exec=" in desktop_file.read_text(encoding="utf-8")
    assert _linux_is_enabled() is True

    ok, msg = _linux_disable()
    assert ok is True
    assert not desktop_file.exists()
    assert _linux_is_enabled() is False


def test_public_autostart_api(monkeypatch, tmp_path: Path) -> None:
    if sys.platform == "darwin":
        plist_file = tmp_path / "com.maurodruwel.ufora-sync.plist"
        monkeypatch.setattr("ufora_sync.autostart._macos_plist_path", lambda: plist_file)
        monkeypatch.setattr("ufora_sync.autostart.subprocess.run", lambda *args, **kwargs: None)
    elif sys.platform == "win32":
        monkeypatch.setattr("ufora_sync.autostart._windows_is_enabled", lambda: False)
        monkeypatch.setattr("ufora_sync.autostart._windows_enable", lambda: (True, "mock enabled"))
        monkeypatch.setattr(
            "ufora_sync.autostart._windows_disable", lambda: (True, "mock disabled")
        )
    else:
        desktop_file = tmp_path / "ufora-sync.desktop"
        monkeypatch.setattr("ufora_sync.autostart._linux_desktop_path", lambda: desktop_file)

    assert is_autostart_enabled() is False
    ok, _ = enable_autostart()
    assert ok is True
    assert is_autostart_enabled() is True
    ok, _ = disable_autostart()
    assert ok is True
    assert is_autostart_enabled() is False
