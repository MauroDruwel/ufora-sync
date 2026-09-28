"""System tray and menu bar application for Ufora Sync."""

from __future__ import annotations

import logging
import subprocess
import sys
import threading
from collections.abc import Callable
from pathlib import Path
from typing import Any

from PIL import Image, ImageDraw

from ufora_sync.config import AppConfig
from ufora_sync.service import SyncService

logger = logging.getLogger("ufora_sync.tray")


def create_tray_icon_image(status: str = "idle") -> Image.Image:
    """Dynamically generate a sleek modern cloud icon with status indicator."""
    size = (64, 64)
    image = Image.new("RGBA", size, (0, 0, 0, 0))
    draw = ImageDraw.Draw(image)

    # Cloud body in sleek blue / indigo
    cloud_color = (66, 133, 244, 255)  # Modern blue
    # Base rounded pill
    draw.rounded_rectangle([12, 28, 52, 50], radius=11, fill=cloud_color)
    # Left puff
    draw.ellipse([16, 20, 36, 40], fill=cloud_color)
    # Right puff
    draw.ellipse([28, 14, 48, 38], fill=cloud_color)

    if status == "syncing":
        # Green pulsing sync badge with white border
        badge_fill = (34, 197, 94, 255)
        draw.ellipse([40, 36, 58, 54], fill=badge_fill, outline=(255, 255, 255, 255), width=2)
        # Inner white sync dot
        draw.ellipse([46, 42, 52, 48], fill=(255, 255, 255, 255))
    elif status == "paused":
        # Yellow pause badge
        badge_fill = (234, 179, 8, 255)
        draw.ellipse([40, 36, 58, 54], fill=badge_fill, outline=(255, 255, 255, 255), width=2)
    elif "error" in status.lower() or "auth" in status.lower() or "failed" in status.lower():
        # Red warning badge
        badge_fill = (239, 68, 68, 255)
        draw.ellipse([40, 36, 58, 54], fill=badge_fill, outline=(255, 255, 255, 255), width=2)
    else:
        # Subtle idle badge (gentle teal)
        badge_fill = (52, 211, 153, 255)
        draw.ellipse([42, 38, 56, 52], fill=badge_fill, outline=(255, 255, 255, 255), width=2)

    return image


def open_folder_in_os(path: Path | str) -> None:
    """Reveal a folder in the native OS file manager."""
    p = str(Path(path).expanduser().resolve())
    if sys.platform == "darwin":
        subprocess.run(["open", p], check=False)
    elif sys.platform == "win32":
        subprocess.run(["explorer", p], check=False)
    else:
        subprocess.run(["xdg-open", p], check=False)


class TrayApp:
    """Manages the system tray / menu bar icon and menu."""

    def __init__(
        self,
        service: SyncService,
        on_open_gui: Callable[[], None] | None = None,
        on_quit: Callable[[], None] | None = None,
    ) -> None:
        self.service = service
        self.on_open_gui = on_open_gui
        self.on_quit_callback = on_quit
        self._icon: Any = None
        self._current_status = "Idle"

        self.service.add_status_listener(self._on_service_status)

    def _on_service_status(self, status: str) -> None:
        self._current_status = status
        if self._icon:
            try:
                kind = "syncing" if "sync" in status.lower() else "idle"
                if "error" in status.lower() or "auth" in status.lower():
                    kind = "error"
                elif self.service.is_paused:
                    kind = "paused"
                self._icon.icon = create_tray_icon_image(kind)
                self._icon.title = f"Ufora Sync — {status}"
            except Exception as e:
                logger.debug("Failed updating tray icon: %s", e)

    def _get_menu_items(self) -> list[Any]:

        import pystray

        status_text = f"● {self._current_status}"
        pause_label = "▶ Resume Syncing" if self.service.is_paused else "⏸ Pause Syncing"

        def _action_settings(icon, item):
            if self.on_open_gui:
                self.on_open_gui()

        def _action_sync_now(icon, item):
            self.service.trigger_sync()

        def _action_open_folder(icon, item):
            config = AppConfig.load()
            open_folder_in_os(config.sync_dir)

        def _action_toggle_pause(icon, item):
            if self.service.is_paused:
                self.service.resume()
            else:
                self.service.pause()

        def _action_quit(icon, item):
            self.stop()
            if self.on_quit_callback:
                self.on_quit_callback()

        return [
            pystray.MenuItem("🎓 Ufora Sync", lambda i, it: None, enabled=False),
            pystray.MenuItem(status_text, lambda i, it: None, enabled=False),
            pystray.Menu.SEPARATOR,
            pystray.MenuItem("⚙️ Settings & Courses…", _action_settings, default=True),
            pystray.MenuItem("🔄 Sync Now", _action_sync_now),
            pystray.MenuItem("📂 Open Sync Folder", _action_open_folder),
            pystray.MenuItem(pause_label, _action_toggle_pause),
            pystray.Menu.SEPARATOR,
            pystray.MenuItem("🚪 Quit Ufora Sync", _action_quit),
        ]

    def run(self) -> None:
        """Run the tray icon event loop (blocking)."""
        import pystray

        self._icon = pystray.Icon(
            name="ufora_sync",
            icon=create_tray_icon_image("idle"),
            title="Ufora Sync",
            menu=pystray.Menu(lambda: self._get_menu_items()),
        )
        self._icon.run()


    def run_detached(self) -> threading.Thread:
        """Run the tray icon in a dedicated background thread."""
        t = threading.Thread(target=self.run, daemon=True, name="UforaTrayThread")
        t.start()
        return t

    def stop(self) -> None:
        if self._icon:
            import contextlib
            with contextlib.suppress(Exception):
                self._icon.stop()
            self._icon = None

