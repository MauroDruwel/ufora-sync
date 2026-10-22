"""Tests for configuration persistence and course toggling."""

from __future__ import annotations

from pathlib import Path

from ufora_sync.config import AppConfig, get_default_config_dir, get_default_sync_dir
from ufora_sync.sync import ConflictStrategy


def test_default_config_properties():
    cfg = AppConfig()
    assert cfg.interval_minutes == 30
    assert cfg.conflict_strategy == ConflictStrategy.DUPLICATE
    assert cfg.duplicate_suffix == "_edited"
    assert isinstance(cfg.enabled_courses, list)
    assert len(cfg.enabled_courses) == 0


def test_course_toggle():
    cfg = AppConfig()
    cid = "1377778"
    assert not cfg.is_course_enabled(cid)

    # Enable
    res = cfg.toggle_course(cid, True)
    assert res is True
    assert cfg.is_course_enabled(cid)

    # Disable
    res = cfg.toggle_course(cid, False)
    assert res is False
    assert not cfg.is_course_enabled(cid)

    # Flip toggle
    cfg.toggle_course(cid)
    assert cfg.is_course_enabled(cid)
    cfg.toggle_course(cid)
    assert not cfg.is_course_enabled(cid)


def test_config_save_and_load(tmp_path: Path, monkeypatch):
    monkeypatch.setattr(
        AppConfig, "get_config_file", classmethod(lambda cls: tmp_path / "config.json")
    )

    cfg = AppConfig(
        sync_dir=str(tmp_path / "Courses"),
        interval_minutes=45,
        enabled_courses=["101", "202"],
        conflict_strategy=ConflictStrategy.SKIP,
        duplicate_suffix="_mine",
    )
    cfg.save()

    loaded = AppConfig.load()
    assert loaded.sync_dir == str(tmp_path / "Courses")
    assert loaded.interval_minutes == 45
    assert loaded.enabled_courses == ["101", "202"]
    assert loaded.conflict_strategy == ConflictStrategy.SKIP
    assert loaded.duplicate_suffix == "_mine"


def test_platform_default_directories():
    config_dir = get_default_config_dir()
    assert isinstance(config_dir, Path)
    sync_dir = get_default_sync_dir()
    assert isinstance(sync_dir, Path)
