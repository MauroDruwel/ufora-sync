"""Tests for background sync service lifecycle and listeners."""

from __future__ import annotations

from pathlib import Path

from ufora_sync.config import AppConfig
from ufora_sync.service import SyncService


def test_service_initial_state(tmp_path: Path):
    cfg = AppConfig(sync_dir=str(tmp_path))
    service = SyncService(config=cfg)

    assert not service.is_syncing
    assert not service.is_paused
    assert service.last_status == "Initialized"


def test_service_listeners():
    service = SyncService()
    statuses = []
    logs = []

    service.add_status_listener(lambda s: statuses.append(s))
    service.add_log_listener(lambda msg: logs.append(msg))

    service._notify_status("TestStatus")
    service._notify_log("TestLog")

    assert "TestStatus" in statuses
    assert "TestLog" in logs


def test_service_pause_and_resume():
    service = SyncService()
    assert not service.is_paused

    service.pause()
    assert service.is_paused
    assert service.last_status == "Paused"

    service.resume()
    assert not service.is_paused
    assert service.last_status == "Resumed"


def test_service_start_stop():
    service = SyncService()
    service.start()
    valid_states = (
        "Running",
        "No courses selected",
        "Auth needed: Not logged in",
        "Syncing…",
    )
    assert any(s in service.last_status for s in valid_states)

    service.stop()
    assert service.last_status == "Stopped"


def test_service_lock_prevents_concurrent_sync(monkeypatch, tmp_path: Path):
    from ufora_sync.lock import ProcessLock

    fake_config_dir = tmp_path / "cfg"
    fake_config_dir.mkdir()
    monkeypatch.setattr("ufora_sync.service.get_default_config_dir", lambda: fake_config_dir)

    lock_file = fake_config_dir / "sync.lock"
    external_lock = ProcessLock(lock_file)
    assert external_lock.acquire() is True

    service = SyncService()
    logs = []
    service.add_log_listener(lambda line: logs.append(line))

    service._do_sync_pass()

    assert any("Another sync pass is currently running" in log for log in logs)
    assert not service.is_syncing

    external_lock.release()
