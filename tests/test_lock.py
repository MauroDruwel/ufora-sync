"""Tests for cross-process file lock."""

from __future__ import annotations

from pathlib import Path

from ufora_sync.lock import ProcessLock


def test_lock_acquire_and_release(tmp_path: Path) -> None:
    lock_file = tmp_path / "test.lock"
    lock = ProcessLock(lock_file)

    assert lock.acquire() is True
    assert lock.is_acquired is True
    assert lock_file.exists()

    lock.release()
    assert lock.is_acquired is False
    assert not lock_file.exists()


def test_lock_context_manager(tmp_path: Path) -> None:
    lock_file = tmp_path / "test.lock"
    with ProcessLock(lock_file) as acquired:
        assert acquired is True
        assert lock_file.exists()

    assert not lock_file.exists()


def test_concurrent_lock_denied(tmp_path: Path) -> None:
    lock_file = tmp_path / "test.lock"
    lock1 = ProcessLock(lock_file)
    lock2 = ProcessLock(lock_file)

    assert lock1.acquire() is True
    # Second lock on same file without release should fail non-blocking
    assert lock2.acquire(blocking=False) is False
    assert lock2.is_acquired is False

    lock1.release()
    # Now second lock can acquire
    assert lock2.acquire() is True
    lock2.release()
