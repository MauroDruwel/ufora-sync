"""Inter-process communication and single-instance coordination for Ufora Sync."""

from __future__ import annotations

import contextlib
import logging
import subprocess
import sys
import threading
import time
from collections.abc import Callable

from ufora_sync.config import get_default_config_dir
from ufora_sync.lock import ProcessLock

logger = logging.getLogger("ufora_sync.ipc")


def get_daemon_lock() -> ProcessLock:
    """Return the process lock for the background sync daemon / tray app."""
    return ProcessLock(get_default_config_dir() / "daemon.lock")


def get_gui_lock() -> ProcessLock:
    """Return the process lock for the Settings GUI window."""
    return ProcessLock(get_default_config_dir() / "gui.lock")


def is_daemon_running() -> bool:
    """Check if the background sync daemon is currently running."""
    lock = get_daemon_lock()
    if lock.acquire():
        lock.release()
        return False
    return True


def is_gui_running() -> bool:
    """Check if the Settings GUI window is currently running."""
    lock = get_gui_lock()
    if lock.acquire():
        lock.release()
        return False
    return True


def request_daemon_sync() -> None:
    """Signal the running background daemon to perform an immediate sync pass."""
    trigger_file = get_default_config_dir() / "trigger.sync"
    try:
        trigger_file.parent.mkdir(parents=True, exist_ok=True)
        trigger_file.write_text(str(time.time()), encoding="utf-8")
    except Exception as exc:
        logger.debug("Failed requesting daemon sync: %s", exc)


def request_gui_open() -> None:
    """Signal the background daemon to open or focus the Settings GUI."""
    trigger_file = get_default_config_dir() / "trigger.gui"
    try:
        trigger_file.parent.mkdir(parents=True, exist_ok=True)
        trigger_file.write_text(str(time.time()), encoding="utf-8")
    except Exception as exc:
        logger.debug("Failed requesting GUI open: %s", exc)


def focus_existing_gui() -> None:
    """Attempt to bring the existing GUI window to the foreground."""
    if sys.platform == "darwin":
        try:
            script = (
                'tell application "System Events" to set frontmost of '
                '(first process whose name contains "Python") to true'
            )
            subprocess.run(["osascript", "-e", script], check=False, capture_output=True)
        except Exception as exc:
            logger.debug("Failed focusing GUI via AppleScript: %s", exc)


class IPCWatcher:
    """Background listener inside the daemon watching for external trigger files."""

    def __init__(
        self,
        on_sync: Callable[[], None] | None = None,
        on_open_gui: Callable[[], None] | None = None,
    ) -> None:
        self.on_sync = on_sync
        self.on_open_gui = on_open_gui
        self._stop_event = threading.Event()
        self._thread: threading.Thread | None = None

    def start(self) -> None:
        if self._thread and self._thread.is_alive():
            return
        self._stop_event.clear()
        self._thread = threading.Thread(
            target=self._watch_loop, daemon=True, name="UforaIPCWatcher"
        )
        self._thread.start()

    def stop(self) -> None:
        self._stop_event.set()
        if self._thread and self._thread.is_alive():
            self._thread.join(timeout=1.0)
        self._thread = None

    def _watch_loop(self) -> None:
        config_dir = get_default_config_dir()
        sync_trigger = config_dir / "trigger.sync"
        gui_trigger = config_dir / "trigger.gui"

        while not self._stop_event.is_set():
            if sync_trigger.exists():
                with contextlib.suppress(Exception):
                    sync_trigger.unlink()
                if self.on_sync:
                    try:
                        self.on_sync()
                    except Exception as e:
                        logger.debug("Error handling sync trigger: %s", e)

            if gui_trigger.exists():
                with contextlib.suppress(Exception):
                    gui_trigger.unlink()
                if self.on_open_gui:
                    try:
                        self.on_open_gui()
                    except Exception as e:
                        logger.debug("Error handling GUI trigger: %s", e)

            self._stop_event.wait(0.3)
