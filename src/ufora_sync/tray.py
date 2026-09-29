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
    """Dynamically generate a crisp native icon with status indicator."""
    if sys.platform == "darwin":
        # Render high-res anti-aliased mask for macOS menu bar (Retina 44x44)
        size = 128
        target_size = 44
        mask = Image.new("L", (size, size), 0)
        draw = ImageDraw.Draw(mask)

        # Clean Apple-style cloud silhouette
        draw.rounded_rectangle([20, 58, 108, 98], radius=20, fill=255)
        draw.ellipse([26, 42, 70, 86], fill=255)
        draw.ellipse([48, 26, 92, 70], fill=255)
        draw.ellipse([72, 46, 104, 78], fill=255)

        st = status.lower()
        if "sync" in st:
            # Cutout circulating sync arrows
            draw.arc([46, 48, 82, 84], start=30, end=190, fill=0, width=6)
            draw.arc([46, 48, 82, 84], start=210, end=370, fill=0, width=6)
            draw.polygon([(46, 68), (56, 64), (52, 76)], fill=0)
            draw.polygon([(82, 64), (72, 68), (76, 56)], fill=0)
        elif "pause" in st:
            # Two vertical pause bars
            draw.rounded_rectangle([56, 54, 62, 78], radius=2, fill=0)
            draw.rounded_rectangle([66, 54, 72, 78], radius=2, fill=0)
        elif "error" in st or "auth" in st or "fail" in st:
            # Exclamation mark
            draw.rounded_rectangle([61, 50, 67, 70], radius=2, fill=0)
            draw.ellipse([61, 74, 67, 80], fill=0)

        small_mask = mask.resize((target_size, target_size), Image.LANCZOS)
        rgba = Image.new("RGBA", (target_size, target_size), (255, 255, 255, 255))
        rgba.putalpha(small_mask)
        return rgba

    # Non-macOS (Windows, Linux): colorful cloud with crisp indicator badge
    size = 128
    target_size = 64
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)

    cloud_color = (66, 133, 244, 255)
    draw.rounded_rectangle([20, 58, 108, 98], radius=20, fill=cloud_color)
    draw.ellipse([26, 42, 70, 86], fill=cloud_color)
    draw.ellipse([48, 26, 92, 70], fill=cloud_color)
    draw.ellipse([72, 46, 104, 78], fill=cloud_color)

    st = status.lower()
    if "sync" in st:
        badge_fill = (34, 197, 94, 255)
    elif "pause" in st:
        badge_fill = (234, 179, 8, 255)
    elif "error" in st or "auth" in st or "fail" in st:
        badge_fill = (239, 68, 68, 255)
    else:
        badge_fill = (52, 211, 153, 255)

    draw.ellipse([80, 70, 114, 104], fill=badge_fill, outline=(255, 255, 255, 255), width=4)
    return img.resize((target_size, target_size), Image.LANCZOS)


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

    def _apply_macos_template_mode(self) -> None:
        """Apply native macOS template styling and Retina 2x sizing to the menu bar icon."""
        if sys.platform != "darwin" or not self._icon or not hasattr(self._icon, "_status_item"):
            return
        try:
            import io

            import AppKit
            import Foundation

            b = io.BytesIO()
            self._icon._icon.save(b, "png")
            data = Foundation.NSData.dataWithBytes_length_(b.getvalue(), len(b.getvalue()))
            ns_img = AppKit.NSImage.alloc().initWithData_(data)
            ns_img.setSize_(AppKit.NSMakeSize(22, 22))
            ns_img.setTemplate_(True)

            button = self._icon._status_item.button()
            if button:
                button.setImage_(ns_img)
                self._icon._icon_image = ns_img
        except Exception as e:
            logger.debug("Failed applying macOS template mode: %s", e)

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
                self._apply_macos_template_mode()
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

        def setup(icon: Any) -> None:
            self._apply_macos_template_mode()

        self._icon = pystray.Icon(
            name="ufora_sync",
            icon=create_tray_icon_image("idle"),
            title="Ufora Sync",
            menu=pystray.Menu(lambda: self._get_menu_items()),
        )
        self._icon.run(setup=setup)

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
