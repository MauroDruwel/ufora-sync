"""Sync engine: list courses/content and download files without overwriting local edits."""

from __future__ import annotations

import contextlib
import hashlib
import html
import json
import re
import shutil
import stat
import subprocess
import sys
import tempfile
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

    sync_descriptions: bool = True
    """Whether to generate README.md files for modules containing descriptions or links."""

    sync_links: bool = True
    """Whether to generate clickable .html shortcut files for online activities & links."""

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
    local_path: str  # relative POSIX path within course directory
    sha256: str
    synced_at: str  # ISO-8601
    remote_modified: str | None = None  # LastModifiedDate from Brightspace TOC

    def to_dict(self) -> dict[str, Any]:
        d: dict[str, Any] = {
            "remote_id": self.remote_id,
            "remote_path": self.remote_path,
            "local_path": self.local_path,
            "sha256": self.sha256,
            "synced_at": self.synced_at,
        }
        if self.remote_modified is not None:
            d["remote_modified"] = self.remote_modified
        return d

    @classmethod
    def from_dict(cls, d: dict[str, Any]) -> SyncedFile:
        return cls(
            remote_id=str(d.get("remote_id", "")),
            remote_path=str(d.get("remote_path", "")),
            local_path=str(d.get("local_path", "")),
            sha256=str(d.get("sha256", "")),
            synced_at=str(d.get("synced_at", "")),
            remote_modified=d.get("remote_modified"),
        )


def safe_rel_path(path: Path | str, base: Path | str) -> str:
    """Safely return a relative POSIX path string, falling back cleanly if not a direct subpath."""
    p = Path(path)
    b = Path(base)
    try:
        return str(p.relative_to(b)).replace("\\", "/")
    except ValueError:
        pass

    b_name = b.name
    parts = p.parts
    if b_name in parts:
        idx = parts.index(b_name)
        rel_parts = parts[idx + 1 :]
        if rel_parts:
            return str(Path(*rel_parts)).replace("\\", "/")
    return p.name


@dataclass
class SyncManifest:
    """Tracks files synced to a local directory to detect local edits."""

    base_dir: Path
    files: dict[str, SyncedFile] = field(default_factory=dict)

    @property
    def _path(self) -> Path:
        return self.base_dir / MANIFEST_FILENAME

    def _to_rel_key(self, path: Path | str, fallback_remote: str | None = None) -> str:
        """Convert any path (absolute or relative) to a normalized POSIX relative path key."""
        p = Path(path)
        if not p.is_absolute():
            return str(p).replace("\\", "/")

        try:
            return str(p.relative_to(self.base_dir)).replace("\\", "/")
        except ValueError:
            pass

        # If it was saved with a previous sync directory, find the course folder name
        course_name = self.base_dir.name
        parts = p.parts
        if course_name in parts:
            idx = parts.index(course_name)
            rel_parts = parts[idx + 1 :]
            if rel_parts:
                return str(Path(*rel_parts)).replace("\\", "/")

        if fallback_remote:
            return str(fallback_remote).replace("\\", "/")

        return p.name

    def load(self) -> None:
        if not self._path.exists():
            return
        try:
            data = json.loads(self._path.read_text(encoding="utf-8"))
            raw_files = data.get("files", {})
            self.files = {}
            for k, v in raw_files.items():
                entry = SyncedFile.from_dict(v)
                rel_key = self._to_rel_key(entry.local_path or k, fallback_remote=entry.remote_path)
                entry.local_path = rel_key
                self.files[rel_key] = entry
                # Also store absolute path key for backward compatibility lookups
                self.files[str(self.base_dir / rel_key)] = entry
        except Exception:
            # Corrupt manifest — start fresh
            self.files = {}

    def save(self) -> None:
        """Atomically persist manifest to prevent file corruption on interrupts."""
        self.base_dir.mkdir(parents=True, exist_ok=True)
        # Store only relative path entries on disk to ensure full portability
        # across directories and cloud storage locations.
        saved_dict: dict[str, Any] = {}
        for k, v in self.files.items():
            if not Path(k).is_absolute():
                d = v.to_dict()
                d["local_path"] = k
                saved_dict[k] = d

        payload = {"files": saved_dict}
        tmp_path = self._path.with_name(f".{self._path.name}.tmp_{time.time_ns()}")
        try:
            tmp_path.write_text(json.dumps(payload, indent=2), encoding="utf-8")
            tmp_path.replace(self._path)
        except Exception:
            if tmp_path.exists():
                tmp_path.unlink(missing_ok=True)
            raise

    def get_by_remote_id(self, remote_id: str) -> SyncedFile | None:
        """Find an existing entry by its Brightspace topic ID."""
        rid = str(remote_id)
        for entry in self.files.values():
            if entry.remote_id == rid:
                return entry
        return None

    def resolve_dest(self, entry: SyncedFile) -> Path:
        """Resolve the destination path inside the current active course directory."""
        rel = self._to_rel_key(entry.local_path, fallback_remote=entry.remote_path)
        return self.base_dir / rel

    def is_locally_edited(self, local_path: Path | str) -> bool:
        """Return True if the file exists and has been changed since last sync."""
        rel_key = self._to_rel_key(local_path)
        if rel_key not in self.files:
            return False
        dest_file = self.base_dir / rel_key
        if not dest_file.exists():
            return False
        # If the file is an evicted cloud placeholder, it cannot be locally edited
        # and opening it would force macOS/OneDrive to re-download (hydrate) it.
        if _is_dataless_placeholder(dest_file):
            return False
        current_sha = _sha256(dest_file)
        return current_sha != self.files[rel_key].sha256

    def record(
        self,
        remote_id: str,
        remote_path: str,
        local_path: Path | str,
        remote_modified: str | None = None,
    ) -> None:
        rel_key = self._to_rel_key(local_path, fallback_remote=remote_path)
        abs_dest = self.base_dir / rel_key

        sha = ""
        if abs_dest.exists() and not _is_dataless_placeholder(abs_dest):
            sha = _sha256(abs_dest)
        elif rel_key in self.files:
            sha = self.files[rel_key].sha256

        entry = SyncedFile(
            remote_id=str(remote_id),
            remote_path=str(remote_path).replace("\\", "/"),
            local_path=rel_key,
            sha256=sha,
            synced_at=datetime.now(UTC).isoformat(),
            remote_modified=remote_modified,
        )
        self.files[rel_key] = entry
        self.files[str(abs_dest)] = entry


def _is_dataless_placeholder(path: Path) -> bool:
    """Return True if the file is an evicted cloud placeholder (e.g. OneDrive 'Free Up Space').

    Prevents triggering unwanted on-demand cloud hydration (re-download)
    when inspecting OneDrive / iCloud / Files On-Demand placeholders.
    """
    try:
        st = path.stat()
        # macOS / Darwin APFS dataless file flag (Files On-Demand / iCloud / OneDrive)
        if hasattr(stat, "SF_DATALESS") and bool(st.st_flags & stat.SF_DATALESS):
            return True
        # Windows Files On-Demand (recall on open/data access or offline)
        if hasattr(st, "st_file_attributes"):
            attrs = st.st_file_attributes
            if bool(attrs & (0x00400000 | 0x00040000 | 0x00001000)):
                return True
    except OSError:
        pass
    return False


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
    """Sanitize course title or folder name for filesystem and cloud storage (OneDrive)."""
    cleaned = re.sub(r'[\\/*?:"<>|]', "-", name).strip().rstrip(" .-")
    return cleaned or "Unnamed Course"


def _sanitize_filename(name: str) -> str:
    """Sanitize filename to prevent OneDrive / macOS sync errors (e.g. trailing spaces/dots)."""
    cleaned = re.sub(r'[\\/*?:"<>|]', "-", name).strip()
    p = Path(cleaned)
    stem = p.stem.rstrip(" .")
    suffix = p.suffix.rstrip(" .")
    if not stem:
        stem = "unnamed"
    return f"{stem}{suffix}"


def _clean_legacy_temp_dirs(directory: Path) -> None:
    """Remove any leftover staging or legacy temporary folders from previous interrupted syncs."""
    if not directory.exists():
        return
    for item in directory.glob("_tmp*"):
        if item.is_dir():
            shutil.rmtree(item, ignore_errors=True)
    for hidden_stray in directory.rglob(".*.tmp_*"):
        with contextlib.suppress(OSError):
            hidden_stray.unlink(missing_ok=True)


def _atomic_install_file(src: Path, dest: Path) -> None:
    """Atomically place a file into dest using a hidden temp file in dest's parent folder."""
    dest.parent.mkdir(parents=True, exist_ok=True)
    tmp_dest = dest.with_name(f".{dest.name}.tmp_{time.time_ns()}")
    shutil.copy2(src, tmp_dest)
    tmp_dest.replace(dest)


def get_token_file() -> Path:
    return Path.home() / ".d2l" / "token.json"


def refresh_session() -> bool:
    """Silently renew the Ufora token in the background using saved SSO cookies.

    Returns True if a fresh token was successfully captured, False otherwise.
    """
    # 1. Try invoking ufora refresh via CLI
    try:
        exe = _ufora_exe()
        res = subprocess.run([exe, "refresh"], capture_output=True, text=True, timeout=45)
        if res.returncode == 0:
            return True
    except Exception:
        pass

    # 2. Try in-process via ufora_cli
    try:
        import importlib

        d2l_entry = importlib.import_module("ufora_cli.d2l_entry")
        d2l_entry._patch_session()
        auth_cmd = importlib.import_module("d2l.commands.auth_cmd")
        return bool(auth_cmd.attempt_auto_login())
    except Exception:
        pass

    return False


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


def ensure_authenticated(min_remaining_seconds: int = 180) -> tuple[bool, str]:
    """Ensure the user is authenticated, silently renewing expired or near-expiry tokens.

    If the token is expired or expires within ``min_remaining_seconds``,
    attempts a silent background renewal using the saved SSO session.
    """
    token_file = get_token_file()
    if not token_file.exists():
        browser_profile = Path.home() / ".d2l" / "browser_profile"
        if browser_profile.exists() and refresh_session():
            return check_auth_status()
        return False, "Not logged in"

    try:
        data = json.loads(token_file.read_text(encoding="utf-8"))
        exp = data.get("exp", 0)
        user = data.get("user_id") or data.get("sub") or "User"
        now = time.time()
        if now + min_remaining_seconds >= exp:
            if refresh_session():
                return check_auth_status()
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
    """Call `ufora download-content` for a single topic or module into a staging directory."""
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

    def _walk(item_list: list[ContentNode]) -> None:
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
    """Download selected topics from a course into base_dir robustly using OS temp staging."""
    cfg = config or SyncConfig()
    result = SyncResult()
    target_subfolder = course_subfolder or course_id
    course_dir = base_dir / target_subfolder
    course_dir.mkdir(parents=True, exist_ok=True)

    # Clean legacy temporary folders from past interrupted runs
    _clean_legacy_temp_dirs(course_dir)

    manifest = SyncManifest(base_dir=course_dir)
    manifest.load()

    for topic_id in topic_ids:
        if on_progress:
            on_progress(f"Downloading item {topic_id}…")

        with tempfile.TemporaryDirectory(prefix=f"ufora_stage_{topic_id}_") as stage_dir_str:
            stage_dir = Path(stage_dir_str)
            try:
                _run_ufora_download(course_id, topic_id, stage_dir)
            except RuntimeError as exc:
                result.errors.append((topic_id, str(exc)))
                if on_progress:
                    on_progress(f"  ✗ Error: {exc}")
                continue

            for src in sorted(stage_dir.rglob("*")):
                if not src.is_file():
                    continue
                relative = src.relative_to(stage_dir)
                clean_parts = [_sanitize_folder_name(p) for p in relative.parts[:-1]]
                clean_name = _sanitize_filename(relative.name)
                dest = course_dir.joinpath(*clean_parts, clean_name)
                remote_path = safe_rel_path(dest, course_dir)

                locally_edited = dest.exists() and manifest.is_locally_edited(dest)

                if locally_edited:
                    strategy = cfg.conflict_strategy
                    if strategy == ConflictStrategy.SKIP:
                        result.skipped_edited.append(str(relative))
                        if on_progress:
                            on_progress(f"  ↷ Kept local edit: {relative}")
                        continue
                    elif strategy == ConflictStrategy.DUPLICATE:
                        edited_dest = cfg.edited_path(dest)
                        shutil.move(str(dest), str(edited_dest))
                        result.skipped_edited.append(str(relative))
                        if on_progress:
                            on_progress(
                                f"  ✎ Stashed edit → {edited_dest.name} | updated {relative}"
                            )
                    elif strategy == ConflictStrategy.OVERWRITE:
                        result.skipped_edited.append(str(relative))
                        if on_progress:
                            on_progress(f"  ⚠ Overwrote local edit: {relative}")

                _atomic_install_file(src, dest)
                manifest.record(topic_id, remote_path, dest)
                result.downloaded.append(str(relative))
                if on_progress and not locally_edited:
                    on_progress(f"  ✓ {relative}")

    manifest.save()
    return result


@dataclass
class FileTopic:
    id: str
    title: str
    module_path: tuple[str, ...] = ()
    remote_modified: str | None = None
    url: str | None = None


def html_to_markdown(raw_html: str) -> str:
    """Convert HTML module and topic descriptions into clean Markdown."""
    if not raw_html:
        return ""
    text = html.unescape(raw_html)
    # Convert links: <a href="...">...</a> -> [...](...)
    text = re.sub(
        r'<a\s+[^>]*href=["\']([^"\']+)["\'][^>]*>(.*?)</a>',
        r"[\2](\1)",
        text,
        flags=re.IGNORECASE | re.DOTALL,
    )
    # Bold / strong
    text = re.sub(
        r"<(?:strong|b)>(.*?)</(?:strong|b)>",
        r"**\1**",
        text,
        flags=re.IGNORECASE | re.DOTALL,
    )
    # Italic / em
    text = re.sub(
        r"<(?:em|i)>(.*?)</(?:em|i)>",
        r"*\1*",
        text,
        flags=re.IGNORECASE | re.DOTALL,
    )
    # List items
    text = re.sub(
        r"<li[^>]*>(.*?)</li>",
        r"- \1\n",
        text,
        flags=re.IGNORECASE | re.DOTALL,
    )
    # Line breaks and paragraphs
    text = re.sub(r"<br\s*/?>", "\n", text, flags=re.IGNORECASE)
    text = re.sub(r"</p>", "\n\n", text, flags=re.IGNORECASE)
    text = re.sub(r"<p[^>]*>", "", text, flags=re.IGNORECASE)
    # Strip any remaining tags
    text = re.sub(r"<[^>]+>", "", text)
    # Clean excessive newlines
    text = re.sub(r"\n{3,}", "\n\n", text)
    return text.strip()


def _resolve_topic_url(course_id: str, topic: dict[str, Any]) -> str:
    """Resolve full URL for any Ufora topic (link, activity, or external URL)."""
    raw_url = str(topic.get("Url") or "").strip()
    tid = str(topic.get("TopicId") or topic.get("Id") or "")
    if raw_url.startswith("http://") or raw_url.startswith("https://"):
        return raw_url
    if raw_url.startswith("/"):
        return f"https://ufora.ugent.be{raw_url}"
    if tid:
        return f"https://ufora.ugent.be/d2l/le/content/{course_id}/viewContent/{tid}/View"
    return "https://ufora.ugent.be"


def _generate_html_shortcut(title: str, url: str) -> str:
    """Generate a lightweight cross-platform HTML redirect shortcut."""
    escaped_title = html.escape(title)
    escaped_url = html.escape(url)
    return (
        '<!DOCTYPE html>\n<html lang="nl">\n<head>\n'
        '  <meta charset="utf-8">\n'
        f"  <title>{escaped_title} - Ufora</title>\n"
        f'  <meta http-equiv="refresh" content="0; url={escaped_url}">\n'
        "</head>\n"
        "<body style=\"font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, "
        'sans-serif; background: #0f172a; color: #f8fafc; padding: 40px; text-align: center;">\n'
        f"  <h2>Opening {escaped_title}…</h2>\n"
        f'  <p><a href="{escaped_url}" style="color: #38bdf8;">'
        "Klik hier als je niet automatisch wordt doorgestuurd.</a></p>\n"
        f'  <script>window.location.href = "{escaped_url}";</script>\n'
        "</body>\n</html>\n"
    )


def fetch_course_toc_and_file_topics(
    course_id: str,
) -> tuple[dict[str, Any] | None, list[FileTopic]]:
    """Extract course TOC and downloadable file topics in a single call."""
    try:
        from ufora_cli.materials import _client_and_resolver, _walk_topics

        client, resolver = _client_and_resolver()
        enrollment = resolver.resolve(str(course_id))
        org_id = enrollment["OrgUnit"]["Id"]
        toc = client.content_toc(org_id)
        if not isinstance(toc, dict):
            return None, []

        file_topics: list[FileTopic] = []
        for path_tuple, topic in _walk_topics(toc):
            is_file = topic.get("TypeIdentifier") == "File" or topic.get("TopicType") == 1
            if not is_file:
                continue
            tid = str(topic.get("TopicId") or topic.get("Id") or "")
            if not tid:
                continue
            title = str(topic.get("Title") or f"topic-{tid}")
            modified = str(topic.get("LastModifiedDate") or "").strip() or None
            url = str(topic.get("Url") or "").strip() or None
            file_topics.append(
                FileTopic(
                    id=tid,
                    title=title,
                    module_path=path_tuple,
                    remote_modified=modified,
                    url=url,
                )
            )
        return toc, file_topics
    except Exception:
        try:
            nodes = list_content(course_id)
            return None, [
                FileTopic(
                    id=n.id,
                    title=n.title,
                    module_path=tuple(n.path.split("/")[:-1]) if "/" in n.path else (),
                )
                for n in collect_file_topics(nodes)
            ]
        except Exception:
            return None, []


def fetch_course_file_topics(course_id: str) -> list[FileTopic]:
    """Extract all downloadable file topics along with their exact Ufora module path."""
    _, topics = fetch_course_toc_and_file_topics(course_id)
    return topics


def sync_course_descriptions_and_links(
    course_id: str,
    course_dir: Path,
    toc: dict[str, Any],
    manifest: SyncManifest,
    cfg: SyncConfig,
    result: SyncResult,
    on_progress: Any = None,
) -> bool:
    """Generate README.md and .html shortcuts for modules and online activities."""
    manifest_dirty = False

    def _walk_module(module: dict[str, Any], path_tuple: tuple[str, ...]) -> None:
        nonlocal manifest_dirty

        title = str(module.get("Title") or "Untitled")
        mod_id = str(module.get("ModuleId") or module.get("Id") or "")
        clean_title = _sanitize_folder_name(title)
        current_path = path_tuple + (clean_title,)

        module_dir = course_dir
        for part in current_path:
            module_dir = module_dir / part

        desc_raw = (
            (module.get("Description") or {}).get("Html")
            or (module.get("Description") or {}).get("Text")
            or ""
        )
        desc_md = html_to_markdown(desc_raw)

        topics = module.get("Topics") or []
        non_files = [
            t for t in topics if t.get("TypeIdentifier") != "File" and t.get("TopicType") != 1
        ]

        # 1. Generate README.md if module has a description or non-file activities
        if cfg.sync_descriptions and (desc_md or non_files):
            module_dir.mkdir(parents=True, exist_ok=True)
            readme_path = module_dir / "README.md"
            rel_readme = safe_rel_path(readme_path, course_dir)

            md_lines = [f"# {title}"]
            if desc_md:
                md_lines.extend(["", desc_md])

            if non_files:
                md_lines.extend(["", "---", "", "## 🔗 Online Activiteiten & Links", ""])
                for t in non_files:
                    ttitle = str(t.get("Title") or "Link")
                    turl = _resolve_topic_url(course_id, t)
                    ttype = str(t.get("TypeIdentifier") or "Link")
                    tdesc_raw = (
                        (t.get("Description") or {}).get("Html")
                        or (t.get("Description") or {}).get("Text")
                        or ""
                    )
                    tdesc_md = html_to_markdown(tdesc_raw)
                    md_lines.append(f"- **[{ttitle}]({turl})** *({ttype})*")
                    if tdesc_md:
                        quoted = "> " + tdesc_md.replace("\n", "\n> ")
                        md_lines.append(quoted)
                    md_lines.append("")

            readme_content = "\n".join(md_lines).strip() + "\n"
            content_bytes = readme_content.encode("utf-8")
            content_sha = hashlib.sha256(content_bytes).hexdigest()

            needs_write = True
            existing = manifest.get_by_remote_id(f"mod_desc_{mod_id}")
            if readme_path.exists() and existing and existing.sha256 == content_sha:
                needs_write = False
                result.skipped_exists.append(str(rel_readme))

            if needs_write:
                tmp_readme = readme_path.with_name(f".README.md.tmp_{time.time_ns()}")
                tmp_readme.write_bytes(content_bytes)
                tmp_readme.replace(readme_path)
                manifest.record(f"mod_desc_{mod_id}", str(rel_readme), readme_path)
                manifest_dirty = True
                result.downloaded.append(str(rel_readme))
                if on_progress:
                    on_progress(f"  ✓ {rel_readme}")

        # 2. Generate .html shortcut files for online activities
        if cfg.sync_links and non_files:
            module_dir.mkdir(parents=True, exist_ok=True)
            for t in non_files:
                ttitle = str(t.get("Title") or "Link")
                tid = str(t.get("TopicId") or t.get("Id") or "")
                turl = _resolve_topic_url(course_id, t)
                clean_link_title = _sanitize_filename(ttitle)
                if not clean_link_title.lower().endswith(".html"):
                    shortcut_name = f"{clean_link_title}.html"
                else:
                    shortcut_name = clean_link_title
                shortcut_path = module_dir / shortcut_name
                rel_shortcut = safe_rel_path(shortcut_path, course_dir)

                shortcut_html = _generate_html_shortcut(ttitle, turl)
                shortcut_bytes = shortcut_html.encode("utf-8")
                shortcut_sha = hashlib.sha256(shortcut_bytes).hexdigest()

                needs_write = True
                existing = manifest.get_by_remote_id(f"link_{tid}")
                if shortcut_path.exists() and existing and existing.sha256 == shortcut_sha:
                    needs_write = False
                    result.skipped_exists.append(str(rel_shortcut))

                if needs_write:
                    tmp_shortcut = shortcut_path.with_name(f".{shortcut_name}.tmp_{time.time_ns()}")
                    tmp_shortcut.write_bytes(shortcut_bytes)
                    tmp_shortcut.replace(shortcut_path)
                    manifest.record(f"link_{tid}", str(rel_shortcut), shortcut_path)
                    manifest_dirty = True
                    result.downloaded.append(str(rel_shortcut))
                    if on_progress:
                        on_progress(f"  ✓ {rel_shortcut}")

        for sub in module.get("Modules") or []:
            _walk_module(sub, current_path)

    for m in toc.get("Modules") or []:
        _walk_module(m, ())

    return manifest_dirty


def sync_course_all(
    course: CourseInfo,
    base_dir: Path,
    *,
    config: SyncConfig | None = None,
    on_progress: Any = None,
) -> SyncResult:
    """Download all file-backed materials for a course, preserving folder structure robustly."""
    if on_progress:
        on_progress(f"Inspecting course tree: {course.name}…")

    toc, file_topics = fetch_course_toc_and_file_topics(course.id)
    if not file_topics and not toc:
        if on_progress:
            on_progress(f"No downloadable files found in {course.name}")
        return SyncResult()

    cfg = config or SyncConfig()
    result = SyncResult()
    course_dir = base_dir / course.folder_name
    course_dir.mkdir(parents=True, exist_ok=True)

    # Sweep and clean legacy temporary staging folders
    _clean_legacy_temp_dirs(course_dir)

    manifest = SyncManifest(base_dir=course_dir)
    manifest.load()

    manifest_dirty = False

    for item in file_topics:
        topic_id = item.id

        # Target subfolder inside course
        target_folder = course_dir
        for subfolder in item.module_path:
            clean_subfolder = _sanitize_folder_name(subfolder)
            target_folder = target_folder / clean_subfolder
        target_folder.mkdir(parents=True, exist_ok=True)

        # Smart incremental check: avoid downloading unchanged files
        existing_entry = manifest.get_by_remote_id(topic_id)
        if existing_entry:
            dest_path = manifest.resolve_dest(existing_entry)
            clean_filename = _sanitize_filename(dest_path.name)
            clean_dest_path = dest_path.with_name(clean_filename)
            if dest_path.name != clean_filename:
                if dest_path.exists() and not clean_dest_path.exists():
                    dest_path.rename(clean_dest_path)
                if clean_dest_path.exists():
                    dest_path = clean_dest_path
                    manifest.record(
                        topic_id,
                        safe_rel_path(dest_path, course_dir),
                        dest_path,
                        remote_modified=existing_entry.remote_modified,
                    )
                    manifest_dirty = True

            if dest_path.exists():
                # If remote_modified matches (or both are unset), nothing changed on remote
                if existing_entry.remote_modified == item.remote_modified:
                    is_edited = manifest.is_locally_edited(dest_path)
                    rel_str = safe_rel_path(dest_path, course_dir)
                    if is_edited:
                        result.skipped_edited.append(rel_str)
                    else:
                        result.skipped_exists.append(rel_str)
                    continue

                is_edited = manifest.is_locally_edited(dest_path)
                # Backfill remote_modified for files downloaded in previous versions
                if (
                    not is_edited
                    and existing_entry.remote_modified is None
                    and item.remote_modified
                ):
                    existing_entry.remote_modified = item.remote_modified
                    manifest_dirty = True
                    result.skipped_exists.append(safe_rel_path(dest_path, course_dir))
                    continue

        if on_progress:
            folder_display = " / ".join(item.module_path)
            where_str = f" [{folder_display}]" if folder_display else ""
            on_progress(f"Downloading item {topic_id}{where_str}…")

        with tempfile.TemporaryDirectory(prefix=f"ufora_stage_{topic_id}_") as stage_dir_str:
            stage_dir = Path(stage_dir_str)
            try:
                _run_ufora_download(course.id, topic_id, stage_dir)
            except RuntimeError as exc:
                result.errors.append((topic_id, str(exc)))
                if on_progress:
                    on_progress(f"  ✗ Error: {exc}")
                continue

            for src in sorted(stage_dir.rglob("*")):
                if not src.is_file():
                    continue

                filename = _sanitize_filename(src.name)
                dest = target_folder / filename
                relative_str = safe_rel_path(dest, course_dir)
                remote_path = relative_str

                locally_edited = dest.exists() and manifest.is_locally_edited(dest)

                if locally_edited:
                    strategy = cfg.conflict_strategy
                    if strategy == ConflictStrategy.SKIP:
                        result.skipped_edited.append(relative_str)
                        if on_progress:
                            on_progress(f"  ↷ Kept local edit: {relative_str}")
                        continue
                    elif strategy == ConflictStrategy.DUPLICATE:
                        edited_dest = cfg.edited_path(dest)
                        shutil.move(str(dest), str(edited_dest))
                        result.skipped_edited.append(relative_str)
                        if on_progress:
                            on_progress(
                                f"  ✎ Stashed edit → {edited_dest.name} | updated {relative_str}"
                            )
                    elif strategy == ConflictStrategy.OVERWRITE:
                        result.skipped_edited.append(relative_str)
                        if on_progress:
                            on_progress(f"  ⚠ Overwrote local edit: {relative_str}")

                _atomic_install_file(src, dest)
                manifest.record(
                    topic_id,
                    remote_path,
                    dest,
                    remote_modified=item.remote_modified,
                )
                manifest_dirty = True
                result.downloaded.append(relative_str)
                if on_progress and not locally_edited:
                    on_progress(f"  ✓ {relative_str}")

    if toc and (cfg.sync_descriptions or cfg.sync_links):
        desc_dirty = sync_course_descriptions_and_links(
            course.id,
            course_dir,
            toc,
            manifest,
            cfg,
            result,
            on_progress=on_progress,
        )
        if desc_dirty:
            manifest_dirty = True

    if manifest_dirty:
        manifest.save()

    if on_progress:
        downloaded = len(result.downloaded)
        skipped = len(result.skipped_exists) + len(result.skipped_edited)
        if downloaded == 0 and skipped > 0:
            on_progress(f"  ✓ All {skipped} item(s) are up to date.")
        elif downloaded > 0:
            on_progress(f"  Summary: {downloaded} downloaded/updated, {skipped} up to date.")

    return result
