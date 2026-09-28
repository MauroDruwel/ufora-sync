"""Tests for the sync manifest, local-edit detection, and conflict strategies."""

from __future__ import annotations

import hashlib
from pathlib import Path

from ufora_sync.sync import ConflictStrategy, SyncConfig, SyncManifest, _sha256

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _write(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(content)


# ---------------------------------------------------------------------------
# SHA-256 helper
# ---------------------------------------------------------------------------


def test_sha256_consistency(tmp_path: Path) -> None:
    f = tmp_path / "test.txt"
    _write(f, b"hello")
    assert _sha256(f) == hashlib.sha256(b"hello").hexdigest()


# ---------------------------------------------------------------------------
# Manifest round-trip
# ---------------------------------------------------------------------------


def test_manifest_roundtrip(tmp_path: Path) -> None:
    manifest = SyncManifest(base_dir=tmp_path)
    f = tmp_path / "slides.pdf"
    _write(f, b"PDF content")
    manifest.record("topic-1", "Slides/slides.pdf", f)
    manifest.save()

    manifest2 = SyncManifest(base_dir=tmp_path)
    manifest2.load()
    assert str(f) in manifest2.files
    assert manifest2.files[str(f)].remote_id == "topic-1"


# ---------------------------------------------------------------------------
# Edit detection
# ---------------------------------------------------------------------------


def test_no_local_edit_detected(tmp_path: Path) -> None:
    manifest = SyncManifest(base_dir=tmp_path)
    f = tmp_path / "notes.txt"
    _write(f, b"original")
    manifest.record("topic-2", "notes.txt", f)
    manifest.save()

    manifest2 = SyncManifest(base_dir=tmp_path)
    manifest2.load()
    assert not manifest2.is_locally_edited(f)


def test_local_edit_detected(tmp_path: Path) -> None:
    manifest = SyncManifest(base_dir=tmp_path)
    f = tmp_path / "notes.txt"
    _write(f, b"original")
    manifest.record("topic-3", "notes.txt", f)
    manifest.save()

    _write(f, b"edited by student!")

    manifest2 = SyncManifest(base_dir=tmp_path)
    manifest2.load()
    assert manifest2.is_locally_edited(f)


def test_unknown_file_not_locally_edited(tmp_path: Path) -> None:
    manifest = SyncManifest(base_dir=tmp_path)
    f = tmp_path / "new_file.pdf"
    _write(f, b"something")
    assert not manifest.is_locally_edited(f)


def test_missing_file_not_locally_edited(tmp_path: Path) -> None:
    manifest = SyncManifest(base_dir=tmp_path)
    f = tmp_path / "missing.pdf"
    _write(f, b"data")
    manifest.record("t", "missing.pdf", f)
    f.unlink()
    assert not manifest.is_locally_edited(f)


# ---------------------------------------------------------------------------
# SyncConfig — conflict strategy defaults
# ---------------------------------------------------------------------------


def test_default_strategy_is_duplicate() -> None:
    cfg = SyncConfig()
    assert cfg.conflict_strategy == ConflictStrategy.DUPLICATE


def test_default_suffix() -> None:
    cfg = SyncConfig()
    assert cfg.duplicate_suffix == "_edited"


# ---------------------------------------------------------------------------
# SyncConfig.edited_path — naming logic
# ---------------------------------------------------------------------------


def test_edited_path_basic(tmp_path: Path) -> None:
    cfg = SyncConfig(duplicate_suffix="_edited")
    original = tmp_path / "lecture.pdf"
    result = cfg.edited_path(original)
    assert result == tmp_path / "lecture_edited.pdf"


def test_edited_path_custom_suffix(tmp_path: Path) -> None:
    cfg = SyncConfig(duplicate_suffix="_mine")
    original = tmp_path / "notes.docx"
    result = cfg.edited_path(original)
    assert result == tmp_path / "notes_mine.docx"


def test_edited_path_avoids_collision(tmp_path: Path) -> None:
    cfg = SyncConfig(duplicate_suffix="_edited")
    original = tmp_path / "slides.pdf"

    # Pre-create the first candidate
    _write(tmp_path / "slides_edited.pdf", b"collision")

    result = cfg.edited_path(original)
    assert result == tmp_path / "slides_edited_1.pdf"


def test_edited_path_avoids_multiple_collisions(tmp_path: Path) -> None:
    cfg = SyncConfig(duplicate_suffix="_edited")
    original = tmp_path / "slides.pdf"

    _write(tmp_path / "slides_edited.pdf", b"collision 0")
    _write(tmp_path / "slides_edited_1.pdf", b"collision 1")

    result = cfg.edited_path(original)
    assert result == tmp_path / "slides_edited_2.pdf"


# ---------------------------------------------------------------------------
# ConflictStrategy constants
# ---------------------------------------------------------------------------


def test_all_strategies_present() -> None:
    assert ConflictStrategy.SKIP in ConflictStrategy.ALL
    assert ConflictStrategy.DUPLICATE in ConflictStrategy.ALL
    assert ConflictStrategy.OVERWRITE in ConflictStrategy.ALL


def test_labels_cover_all_strategies() -> None:
    for strategy in ConflictStrategy.ALL:
        assert strategy in ConflictStrategy.LABELS
        assert ConflictStrategy.LABELS[strategy]  # non-empty label

