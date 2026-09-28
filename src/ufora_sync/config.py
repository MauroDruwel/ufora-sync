"""Configuration management for Ufora Sync with cross-platform persistence."""

from __future__ import annotations

import json
import os
import sys
from dataclasses import asdict, dataclass, field
from pathlib import Path

from ufora_sync.sync import ConflictStrategy


def get_default_config_dir() -> Path:
    """Return the platform-appropriate configuration directory."""
    if sys.platform == "win32":
        base = os.environ.get("APPDATA")
        if base:
            return Path(base) / "ufora-sync"
        return Path.home() / "AppData" / "Roaming" / "ufora-sync"
    elif sys.platform == "darwin":
        return Path.home() / "Library" / "Application Support" / "ufora-sync"
    else:
        # Linux / BSD / POSIX
        xdg = os.environ.get("XDG_CONFIG_HOME")
        if xdg:
            return Path(xdg) / "ufora-sync"
        return Path.home() / ".config" / "ufora-sync"


def get_default_sync_dir() -> Path:
    """Return the default student sync directory."""
    return Path.home() / "Documents" / "Ufora"


@dataclass
class AppConfig:
    """User configuration for Ufora Sync service and desktop GUI."""

    sync_dir: str = field(default_factory=lambda: str(get_default_sync_dir()))
    interval_minutes: int = 30
    enabled_courses: list[str] = field(default_factory=list)  # Course IDs to sync
    conflict_strategy: str = ConflictStrategy.DUPLICATE
    duplicate_suffix: str = "_edited"
    auto_start_tray: bool = True
    last_sync_time: str | None = None
    last_sync_status: str = "Never synced"

    @classmethod
    def get_config_file(cls) -> Path:
        config_dir = get_default_config_dir()
        config_dir.mkdir(parents=True, exist_ok=True)
        return config_dir / "config.json"

    @classmethod
    def load(cls) -> AppConfig:
        path = cls.get_config_file()
        if not path.exists():
            instance = cls()
            instance.save()
            return instance

        try:
            data = json.loads(path.read_text(encoding="utf-8"))
            valid_keys = cls.__dataclass_fields__.keys()
            filtered = {k: v for k, v in data.items() if k in valid_keys}
            return cls(**filtered)
        except Exception:
            # Fallback on corrupt file
            return cls()

    def save(self) -> None:
        path = self.get_config_file()
        path.parent.mkdir(parents=True, exist_ok=True)
        data = asdict(self)
        path.write_text(json.dumps(data, indent=2), encoding="utf-8")

    def is_course_enabled(self, course_id: str) -> bool:
        return str(course_id) in self.enabled_courses

    def toggle_course(self, course_id: str, enable: bool | None = None) -> bool:
        cid = str(course_id)
        if enable is None:
            enable = cid not in self.enabled_courses

        if enable and cid not in self.enabled_courses:
            self.enabled_courses.append(cid)
        elif not enable and cid in self.enabled_courses:
            self.enabled_courses.remove(cid)
        return enable
