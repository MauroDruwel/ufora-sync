"""OneDrive-style background sync service for UGent Ufora."""

from __future__ import annotations

import logging
import threading
import time
from collections.abc import Callable
from datetime import datetime
from pathlib import Path

from ufora_sync.config import AppConfig, get_default_config_dir
from ufora_sync.lock import ProcessLock
from ufora_sync.sync import (
    SyncConfig,
    SyncResult,
    check_auth_status,
    list_courses,
    sync_course_all,
)

logger = logging.getLogger("ufora_sync.service")


class SyncService:
    """Continuous background sync daemon."""

    def __init__(self, config: AppConfig | None = None) -> None:
        self.config = config or AppConfig.load()
        self._stop_event = threading.Event()
        self._sync_trigger = threading.Event()
        self._thread: threading.Thread | None = None
        self._lock = threading.Lock()

        self.is_syncing = False
        self.is_paused = False
        self.last_status = "Initialized"
        self.last_error: str | None = None

        # Callbacks: on_status(status_str), on_log(text), on_sync_done(result)
        self.status_listeners: list[Callable[[str], None]] = []
        self.log_listeners: list[Callable[[str], None]] = []
        self.completion_listeners: list[Callable[[SyncResult], None]] = []

    def add_status_listener(self, callback: Callable[[str], None]) -> None:
        self.status_listeners.append(callback)

    def add_log_listener(self, callback: Callable[[str], None]) -> None:
        self.log_listeners.append(callback)

    def add_completion_listener(self, callback: Callable[[SyncResult], None]) -> None:
        self.completion_listeners.append(callback)

    def _notify_status(self, status: str) -> None:
        self.last_status = status
        for cb in list(self.status_listeners):
            try:
                cb(status)
            except Exception as e:
                logger.debug("Error in status callback: %s", e)

    def _notify_log(self, text: str) -> None:
        for cb in list(self.log_listeners):
            try:
                cb(text)
            except Exception as e:
                logger.debug("Error in log callback: %s", e)

    def start(self) -> None:
        """Start the background service thread."""
        with self._lock:
            if self._thread and self._thread.is_alive():
                return
            self._stop_event.clear()
            self._sync_trigger.clear()
            self._thread = threading.Thread(
                target=self._run_loop, daemon=True, name="UforaSyncDaemon"
            )
            self._thread.start()
            self._notify_status("Running")
            self._notify_log("Ufora background sync daemon started.")

    def stop(self) -> None:
        """Stop the background service thread."""
        self._stop_event.set()
        self._sync_trigger.set()
        if self._thread and self._thread.is_alive():
            self._thread.join(timeout=3.0)
        self._notify_status("Stopped")
        self._notify_log("Ufora background sync daemon stopped.")

    def trigger_sync(self) -> None:
        """Request an immediate sync iteration."""
        self._sync_trigger.set()

    def pause(self) -> None:
        self.is_paused = True
        self._notify_status("Paused")
        self._notify_log("Syncing paused.")

    def resume(self) -> None:
        self.is_paused = False
        self._notify_status("Resumed")
        self._notify_log("Syncing resumed.")
        self.trigger_sync()

    def _run_loop(self) -> None:
        """Main periodic sync loop."""
        # Initial run on service start after brief warmup
        time.sleep(1.0)
        if not self._stop_event.is_set():
            self._do_sync_pass()

        while not self._stop_event.is_set():
            self.config = AppConfig.load()
            interval_sec = max(60, self.config.interval_minutes * 60)

            # Wait for either interval timeout or manual sync trigger
            self._sync_trigger.wait(timeout=interval_sec)
            self._sync_trigger.clear()

            if self._stop_event.is_set():
                break

            if not self.is_paused:
                self._do_sync_pass()

    def _do_sync_pass(self) -> None:
        """Execute a single pass over all enabled courses."""
        with self._lock:
            if self.is_syncing:
                return
            self.is_syncing = True

        lock = ProcessLock(get_default_config_dir() / "sync.lock")
        if not lock.acquire(blocking=False):
            self._notify_log("Another sync pass is currently running. Skipping.")
            with self._lock:
                self.is_syncing = False
            return

        self.config = AppConfig.load()
        sync_dir = Path(self.config.sync_dir).expanduser()
        sync_dir.mkdir(parents=True, exist_ok=True)

        is_auth, auth_msg = check_auth_status()
        if not is_auth:
            self._notify_status(f"Auth needed: {auth_msg}")
            self._notify_log(f"⚠ {auth_msg}. Run 'ufora login' or use Login button in Settings.")
            lock.release()
            with self._lock:
                self.is_syncing = False
            return

        enabled_ids = set(self.config.enabled_courses)
        if not enabled_ids:
            self._notify_status("No courses selected")
            self._notify_log("No courses enabled for sync. Select courses in the Settings window.")
            lock.release()
            with self._lock:
                self.is_syncing = False
            return

        self._notify_status("Syncing…")
        timestamp_str = datetime.now().strftime("%H:%M:%S")
        self._notify_log(f"[{timestamp_str}] Starting sync for {len(enabled_ids)} course(s)…")

        total_result = SyncResult()
        try:
            courses = list_courses()
            matched = [c for c in courses if c.id in enabled_ids]

            cfg = SyncConfig(
                conflict_strategy=self.config.conflict_strategy,
                duplicate_suffix=self.config.duplicate_suffix,
            )

            for course in matched:
                if self._stop_event.is_set() or self.is_paused:
                    break

                self._notify_status(f"Syncing: {course.name[:25]}…")
                res = sync_course_all(
                    course,
                    sync_dir,
                    config=cfg,
                    on_progress=self._notify_log,
                )
                total_result.downloaded.extend(res.downloaded)
                total_result.skipped_edited.extend(res.skipped_edited)
                total_result.skipped_exists.extend(res.skipped_exists)
                total_result.errors.extend(res.errors)

            # Update config record
            now_str = datetime.now().strftime("%Y-%m-%d %H:%M")
            self.config.last_sync_time = now_str
            if total_result.errors:
                self.config.last_sync_status = f"Finished with {len(total_result.errors)} error(s)"
                self._notify_status(f"Finished with errors ({len(total_result.errors)})")
            else:
                self.config.last_sync_status = "Up to date"
                self._notify_status("Up to date")

            self.config.save()

            summary = (
                f"Sync complete at {now_str}: {len(total_result.downloaded)} downloaded, "
                f"{len(total_result.skipped_edited)} kept local edits, "
                f"{len(total_result.errors)} errors."
            )
            self._notify_log(summary)

            for cb in list(self.completion_listeners):
                try:
                    cb(total_result)
                except Exception as e:
                    logger.debug("Error in completion callback: %s", e)

        except Exception as exc:
            self.last_error = str(exc)
            self._notify_status("Sync failed")
            self._notify_log(f"✗ Sync error: {exc}")
        finally:
            lock.release()
            with self._lock:
                self.is_syncing = False
