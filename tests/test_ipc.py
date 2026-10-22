"""Unit tests for IPC and single-instance coordination module."""

from __future__ import annotations

import time
from pathlib import Path

from ufora_sync.ipc import (
    IPCWatcher,
    get_daemon_lock,
    get_gui_lock,
    is_daemon_running,
    is_gui_running,
    request_daemon_sync,
    request_gui_open,
)


def test_daemon_and_gui_locks(monkeypatch, tmp_path: Path):
    monkeypatch.setattr("ufora_sync.ipc.get_default_config_dir", lambda: tmp_path)

    assert not is_daemon_running()
    assert not is_gui_running()

    dlock = get_daemon_lock()
    assert dlock.acquire() is True
    assert is_daemon_running() is True

    glock = get_gui_lock()
    assert glock.acquire() is True
    assert is_gui_running() is True

    dlock.release()
    assert is_daemon_running() is False

    glock.release()
    assert is_gui_running() is False


def test_ipc_watcher_events(monkeypatch, tmp_path: Path):
    monkeypatch.setattr("ufora_sync.ipc.get_default_config_dir", lambda: tmp_path)

    sync_called = []
    gui_called = []

    watcher = IPCWatcher(
        on_sync=lambda: sync_called.append(True),
        on_open_gui=lambda: gui_called.append(True),
    )
    watcher.start()

    try:
        request_daemon_sync()
        time.sleep(0.5)
        assert len(sync_called) >= 1

        request_gui_open()
        time.sleep(0.5)
        assert len(gui_called) >= 1
    finally:
        watcher.stop()
