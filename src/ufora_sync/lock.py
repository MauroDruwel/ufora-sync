"""Cross-process mutual exclusion file locking."""

from __future__ import annotations

import contextlib
import os
import sys
from pathlib import Path
from typing import Any


class ProcessLock:
    """Inter-process file lock to prevent concurrent sync operations."""

    def __init__(self, lock_file: Path | str) -> None:
        self.lock_path = Path(lock_file)
        self._fd: int | None = None
        self.is_acquired = False

    def acquire(self, blocking: bool = False) -> bool:
        """Attempt to acquire the lock. Returns True if acquired, False otherwise."""
        if self.is_acquired:
            return True

        self.lock_path.parent.mkdir(parents=True, exist_ok=True)
        try:
            self._fd = os.open(
                str(self.lock_path),
                os.O_CREAT | os.O_RDWR | os.O_TRUNC,
                0o644,
            )
            if sys.platform != "win32":
                import fcntl

                flags = fcntl.LOCK_EX
                if not blocking:
                    flags |= fcntl.LOCK_NB
                fcntl.flock(self._fd, flags)
            else:
                import msvcrt

                mode = msvcrt.LK_NBLCK if not blocking else msvcrt.LK_LOCK
                msvcrt.locking(self._fd, mode, 1)

            os.write(self._fd, f"{os.getpid()}\n".encode())
            self.is_acquired = True
            return True
        except (OSError, BlockingIOError, PermissionError):
            if self._fd is not None:
                with contextlib.suppress(OSError):
                    os.close(self._fd)
                self._fd = None
            self.is_acquired = False
            return False

    def release(self) -> None:
        """Release the lock and clean up the lock file."""
        if not self.is_acquired and self._fd is None:
            return

        if self._fd is not None:
            with contextlib.suppress(OSError):
                if sys.platform != "win32":
                    import fcntl

                    fcntl.flock(self._fd, fcntl.LOCK_UN)
                else:
                    import msvcrt

                    msvcrt.locking(self._fd, msvcrt.LK_UNLCK, 1)
                os.close(self._fd)
            self._fd = None

        self.is_acquired = False
        with contextlib.suppress(OSError):
            self.lock_path.unlink(missing_ok=True)

    def __enter__(self) -> bool:
        return self.acquire(blocking=False)

    def __exit__(self, exc_type: Any, exc_val: Any, exc_tb: Any) -> None:
        self.release()
