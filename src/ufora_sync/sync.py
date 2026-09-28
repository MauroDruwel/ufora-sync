"""Sync engine: list courses/content and download files without overwriting local edits."""

from __future__ import annotations

import hashlib
import json
import re
import shutil
import subprocess
import sys
import time
from dataclasses import dataclass, field
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

# ---------------------------------------------------------------------------
# Data classes & Constants
# ---------------------------------------------------------------------------

MANIFEST_FILENAME = ".ufora_sync_manifest.json"


class ConflictStrategy:
    """How to handle a file that exists locally AND has been edited since last sync.

    SKIP      – leave the local version alone, skip downloading the new one.
    DUPLICATE – rename the local edit to '<stem><suffix><ext>', then download
                the fresh version from the prof into the original path.
    OVERWRITE – discard the local edit and always download the prof version.
    """

    SKIP = "skip"
    DUPLICATE = "duplicate"
    OVERWRITE = "overwrite"

    ALL = (SKIP, DUPLICATE, OVERWRITE)
    LABELS = {
        SKIP: "Skip (keep my edit, ignore prof update)",
        DUPLICATE: "Duplicate (keep my edit + download prof version)",
        OVERWRITE: "Overwrite (always use prof version)",
    }


@dataclass
class SyncConfig:
    """User-configurable options for the sync engine."""

    conflict_strategy: str = ConflictStrategy.DUPLICATE
    """What to do when a locally edited file has a new version on Ufora."""

    duplicate_suffix: str = "_edited"
    """Suffix inserted before the file extension for the user's copy.
    e.g. 'notes.pdf' → 'notes_edited.pdf'  (when strategy=DUPLICATE)
    """

    def edited_path(self, path: Path) -> Path:
        """Return the 'edited' sibling path for *path*, avoiding name collisions."""
        candidate = path.with_stem(path.stem + self.duplicate_suffix)
        counter = 1
        while candidate.exists():
            candidate = path.with_stem(f"{path.stem}{self.duplicate_suffix}_{counter}")
            counter += 1
        return candidate


@dataclass
class SyncedFile:
    """Record of a file that was downloaded from Ufora."""

    remote_id: str
    remote_path: str  # e.g. "Slides / Lecture 1.pdf"
    local_path: str  # absolute path on disk
    sha256: str
    synced_at: str  # ISO-8601

    def to_dict(self) -> dict[str, str]:
        return {
            "remote_id": self.remote_id,
            "remote_path": self.remote_path,
            "local_path": self.local_path,
            "sha256": self.sha256,
            "synced_at": self.synced_at,
        }

    @classmethod
    def from_dict(cls, d: dict[str, str]) -> SyncedFile:
        return cls(**d)


@dataclass
class SyncManifest:
    """Tracks files synced to a local directory to detect local edits."""

    base_dir: Path
    files: dict[str, SyncedFile] = field(default_factory=dict)  # key = local_path

    @property
    def _path(self) -> Path:
        return self.base_dir / MANIFEST_FILENAME

    def load(self) -> None:
        if not self._path.exists():
            return
        try:
            data = json.loads(self._path.read_text(encoding="utf-8"))
            self.files = {k: SyncedFile.from_dict(v) for k, v in data.get("files", {}).items()}
        except Exception:
            # Corrupt manifest — start fresh
            self.files = {}

    def save(self) -> None:
        self.base_dir.mkdir(parents=True, exist_ok=True)
        payload = {"files": {k: v.to_dict() for k, v in self.files.items()}}
        self._path.write_text(json.dumps(payload, indent=2), encoding="utf-8")

    def is_locally_edited(self, local_path: Path) -> bool:
        """Return True if the file exists and has been changed since last sync."""
        key = str(local_path)
        if key not in self.files:
            return False
        if not local_path.exists():
            return False
        current_sha = _sha256(local_path)
        return current_sha != self.files[key].sha256

    def record(self, remote_id: str, remote_path: str, local_path: Path) -> None:
        key = str(local_path)
        self.files[key] = SyncedFile(
            remote_id=remote_id,
            remote_path=remote_path,
            local_path=key,
            sha256=_sha256(local_path),
            synced_at=datetime.now(UTC).isoformat(),
        )


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(65536), b""):
            h.update(chunk)
    return h.hexdigest()


def _sanitize_folder_name(name: str) -> str:
    """Sanitize course title for filesystem usage."""
    cleaned = re.sub(r'[\\/*?:"<>|]', "-", name).strip()
    return cleaned or "Unnamed Course"


def get_token_file() -> Path:
    return Path.home() / ".d2l" / "token.json"


def check_auth_status() -> tuple[bool, str]:
    """Fast local check of UGent Ufora authentication without hanging child processes."""
    token_file = get_token_file()
    if not token_file.exists():
        return False, "Not logged in"

    try:
        data = json.loads(token_file.read_text(encoding="utf-8"))
        exp = data.get("exp", 0)
        user = data.get("user_id") or data.get("sub") or "User"
        now = time.time()
        if now > exp:
            return False, "Session expired"
        return True, f"Logged in ({user})"
    except Exception as exc:
        return False, f"Token error ({exc})"


def get_student_name() -> str:
    """Return the student's real name if authenticated, caching in ~/.d2l/profile.json."""
    token_file = get_token_file()
    if not token_file.exists():
        return ""

    profile_file = Path.home() / ".d2l" / "profile.json"
    if profile_file.exists():
        try:
            p = json.loads(profile_file.read_text(encoding="utf-8"))
            first = p.get("FirstName", "").strip()
            last = p.get("LastName", "").strip()
            if first or last:
                return f"{first} {last}".strip()
        except Exception:
            pass

    is_auth, _ = check_auth_status()
    if is_auth:
        try:
            info = _run_ufora_json("whoami")
            if isinstance(info, dict):
                first = info.get("FirstName", "").strip()
                last = info.get("LastName", "").strip()
                name = f"{first} {last}".strip()
                if name:
                    import contextlib

                    with contextlib.suppress(Exception):
                        profile_file.write_text(json.dumps(info), encoding="utf-8")
                    return name
        except Exception:
            pass


    try:
        data = json.loads(token_file.read_text(encoding="utf-8"))
        return str(data.get("user_id") or data.get("sub") or "")
    except Exception:
        return ""



def _ufora_exe() -> str:
    """Locate the installed `ufora` CLI executable."""
    exe = shutil.which("ufora")
    if not exe:
        # Fallback to python bin directory
        fallback = Path(sys.executable).parent / "ufora"
        if fallback.exists():
            return str(fallback)
        raise RuntimeError(
            "The 'ufora' command was not found on PATH.\n"
            "Install it first:  pip3 install -e /path/to/ufora-ai"
        )
    return exe


def _run_ufora_json(*args: str) -> Any:
    """Run `ufora --json <args>` and return parsed JSON, or raise RuntimeError."""
    cmd = [_ufora_exe(), "--json", *args]
    result = subprocess.run(
        cmd,
        capture_output=True,
        text=True,
        timeout=120,
    )
    if result.returncode != 0:
        detail = (result.stderr or result.stdout or "unknown error").strip()
        raise RuntimeError(f"ufora command failed: {detail}")
    payload = result.stdout.strip()
    if not payload:
        return None
    return json.loads(payload)


def _run_ufora_download(course_id: str, topic_id: str, out_dir: Path) -> None:
    """Call `ufora download-content` for a single topic or module."""
    cmd = [
        _ufora_exe(),
        "download-content",
        course_id,
        str(topic_id),
        "-o",
        str(out_dir),
    ]
    result = subprocess.run(
        cmd,
        capture_output=True,
        text=True,
        timeout=120,
    )
    if result.returncode != 0:
        detail = (result.stderr or result.stdout or "unknown error").strip()
        raise RuntimeError(f"Download failed: {detail}")


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------


@dataclass
class CourseInfo:
    id: str
    name: str
    code: str

    @property
    def folder_name(self) -> str:
        return _sanitize_folder_name(self.name)


@dataclass
class ContentNode:
    """A node in the Ufora content tree (module or topic/file)."""

    id: str
    title: str
    kind: str  # "module" | "file" | "link" | "html"
    children: list[ContentNode] = field(default_factory=list)
    path: str = ""  # breadcrumb path for display


def list_courses() -> list[CourseInfo]:
    """Return current-year courses from Ufora."""
    raw = _run_ufora_json("courses")
    if not raw:
        return []
    courses = []
    for item in raw:
        org = item.get("OrgUnit") or {}
        cid = str(org.get("Id") or "")
        name = str(org.get("Name") or cid)
        code = str(org.get("Code") or "")
        if cid:
            courses.append(CourseInfo(id=cid, name=name, code=code))
    return courses


def list_content(course_id: str) -> list[ContentNode]:
    """Return the nested content tree for a course."""
    raw = _run_ufora_json("content", course_id)
    if not raw:
        return []
    return _parse_nodes(raw, parent_path="")


def _parse_nodes(items: list[dict[str, Any]], parent_path: str) -> list[ContentNode]:
    """Recursively parse Brightspace content tree."""
    nodes: list[ContentNode] = []
    if not isinstance(items, list):
        return nodes

    for item in items:
        title = str(item.get("Title") or item.get("title") or "Untitled")
        node_id = str(item.get("Id") or item.get("id") or "")

        type_code = item.get("Type")
        if type_code == 0 or "Structure" in item:
            kind = "module"
        elif type_code == 1:
            kind = "file"
        else:
            kind = "file"

        path = f"{parent_path}/{title}".lstrip("/")
        node = ContentNode(id=node_id, title=title, kind=kind, path=path)

        children_raw = item.get("Structure") or []
        if children_raw:
            node.children = _parse_nodes(children_raw, path)

        nodes.append(node)
    return nodes


def collect_file_topics(nodes: list[ContentNode]) -> list[ContentNode]:
    """Flatten and extract all file-backed topics with IDs from content tree."""
    files: list[ContentNode] = []

    def _walk(item_list: list[ContentNode]):
        for node in item_list:
            if node.kind == "file" and node.id:
                files.append(node)
            if node.children:
                _walk(node.children)

    _walk(nodes)
    return files


@dataclass
class SyncResult:
    downloaded: list[str] = field(default_factory=list)
    skipped_exists: list[str] = field(default_factory=list)
    skipped_edited: list[str] = field(default_factory=list)
    errors: list[tuple[str, str]] = field(default_factory=list)

    @property
    def total(self) -> int:
        return (
            len(self.downloaded)
            + len(self.skipped_exists)
            + len(self.skipped_edited)
            + len(self.errors)
        )


def sync_topics(
    course_id: str,
    topic_ids: list[str],
    base_dir: Path,
    *,
    course_subfolder: str | None = None,
    config: SyncConfig | None = None,
    on_progress: Any = None,  # callable(message: str)
) -> SyncResult:
    """Download selected topics from a course into base_dir."""
    cfg = config or SyncConfig()
    result = SyncResult()
    target_subfolder = course_subfolder or course_id
    course_dir = base_dir / target_subfolder
    course_dir.mkdir(parents=True, exist_ok=True)

    manifest = SyncManifest(base_dir=course_dir)
    manifest.load()

    for topic_id in topic_ids:
        if on_progress:
            on_progress(f"Downloading item {topic_id}…")

        tmp_dir = course_dir / f"_tmp_{topic_id}"
        try:
            _run_ufora_download(course_id, topic_id, tmp_dir)
        except RuntimeError as exc:
            result.errors.append((topic_id, str(exc)))
            if on_progress:
                on_progress(f"  ✗ Error: {exc}")
            continue

        for src in sorted(tmp_dir.rglob("*")):
            if not src.is_file():
                continue
            relative = src.relative_to(tmp_dir)
            dest = course_dir / relative
            dest.parent.mkdir(parents=True, exist_ok=True)
            remote_path = str(relative)

            locally_edited = dest.exists() and manifest.is_locally_edited(dest)

            if locally_edited:
                strategy = cfg.conflict_strategy

                if strategy == ConflictStrategy.SKIP:
                    result.skipped_edited.append(str(relative))
                    if on_progress:
                        on_progress(f"  ↷ Kept local edit: {relative}")
                    src.unlink()
                    continue

                elif strategy == ConflictStrategy.DUPLICATE:
                    edited_dest = cfg.edited_path(dest)
                    shutil.move(str(dest), str(edited_dest))
                    result.skipped_edited.append(str(relative))
                    if on_progress:
                        on_progress(f"  ✎ Stashed edit → {edited_dest.name} | updated {relative}")

                elif strategy == ConflictStrategy.OVERWRITE:
                    result.skipped_edited.append(str(relative))
                    if on_progress:
                        on_progress(f"  ⚠ Overwrote local edit: {relative}")

            shutil.copy2(src, dest)
            manifest.record(topic_id, remote_path, dest)
            result.downloaded.append(str(relative))
            if on_progress and not locally_edited:
                on_progress(f"  ✓ {relative}")
            src.unlink()

        shutil.rmtree(tmp_dir, ignore_errors=True)


    manifest.save()
    return result


@dataclass
class FileTopic:
    id: str
    title: str
    module_path: tuple[str, ...] = ()


def fetch_course_file_topics(course_id: str) -> list[FileTopic]:
    """Extract all downloadable file topics along with their exact Ufora module path."""
    try:
        from ufora_cli.materials import _client_and_resolver, _walk_topics

        client, resolver = _client_and_resolver()
        enrollment = resolver.resolve(str(course_id))
        org_id = enrollment["OrgUnit"]["Id"]
        toc = client.content_toc(org_id)
        if not isinstance(toc, dict):
            return []

        file_topics: list[FileTopic] = []
        for path_tuple, topic in _walk_topics(toc):
            is_file = topic.get("TypeIdentifier") == "File" or topic.get("TopicType") == 1
            if not is_file:
                continue
            tid = str(topic.get("TopicId") or topic.get("Id") or "")
            if not tid:
                continue
            title = str(topic.get("Title") or f"topic-{tid}")
            file_topics.append(FileTopic(id=tid, title=title, module_path=path_tuple))
        return file_topics
    except Exception:
        try:
            nodes = list_content(course_id)
            return [
                FileTopic(
                    id=n.id,
                    title=n.title,
                    module_path=tuple(n.path.split("/")[:-1]) if "/" in n.path else (),
                )
                for n in collect_file_topics(nodes)
            ]
        except Exception:
            return []


def sync_course_all(
    course: CourseInfo,
    base_dir: Path,
    *,
    config: SyncConfig | None = None,
    on_progress: Any = None,
) -> SyncResult:
    """Download all file-backed materials for a complete course, preserving folder structure."""
    if on_progress:
        on_progress(f"Inspecting course tree: {course.name}…")

    file_topics = fetch_course_file_topics(course.id)
    if not file_topics:
        if on_progress:
            on_progress(f"No downloadable files found in {course.name}")
        return SyncResult()

    cfg = config or SyncConfig()
    result = SyncResult()
    course_dir = base_dir / course.folder_name
    course_dir.mkdir(parents=True, exist_ok=True)

    manifest = SyncManifest(base_dir=course_dir)
    manifest.load()

    for item in file_topics:
        topic_id = item.id
        # Build target directory preserving Ufora module structure
        target_folder = course_dir
        for subfolder in item.module_path:
            clean_subfolder = _sanitize_folder_name(subfolder)
            target_folder = target_folder / clean_subfolder
        target_folder.mkdir(parents=True, exist_ok=True)

        if on_progress:
            folder_display = " / ".join(item.module_path)
            where_str = f" [{folder_display}]" if folder_display else ""
            on_progress(f"Downloading item {topic_id}{where_str}…")

        tmp_dir = course_dir / f"_tmp_{topic_id}"
        try:
            _run_ufora_download(course.id, topic_id, tmp_dir)
        except RuntimeError as exc:
            result.errors.append((topic_id, str(exc)))
            if on_progress:
                on_progress(f"  ✗ Error: {exc}")
            continue

        for src in sorted(tmp_dir.rglob("*")):
            if not src.is_file():
                continue

            filename = src.name
            dest = target_folder / filename
            relative = dest.relative_to(course_dir)
            remote_path = str(relative)

            locally_edited = dest.exists() and manifest.is_locally_edited(dest)

            if locally_edited:
                strategy = cfg.conflict_strategy
                if strategy == ConflictStrategy.SKIP:
                    result.skipped_edited.append(str(relative))
                    if on_progress:
                        on_progress(f"  ↷ Kept local edit: {relative}")
                    src.unlink()
                    continue
                elif strategy == ConflictStrategy.DUPLICATE:
                    edited_dest = cfg.edited_path(dest)
                    shutil.move(str(dest), str(edited_dest))
                    result.skipped_edited.append(str(relative))
                    if on_progress:
                        on_progress(f"  ✎ Stashed edit → {edited_dest.name} | updated {relative}")
                elif strategy == ConflictStrategy.OVERWRITE:
                    result.skipped_edited.append(str(relative))
                    if on_progress:
                        on_progress(f"  ⚠ Overwrote local edit: {relative}")

            shutil.copy2(src, dest)
            manifest.record(topic_id, remote_path, dest)
            result.downloaded.append(str(relative))
            if on_progress and not locally_edited:
                on_progress(f"  ✓ {relative}")
            src.unlink()

        shutil.rmtree(tmp_dir, ignore_errors=True)

    manifest.save()
    return result

