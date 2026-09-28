"""Entrypoint and CLI router for Ufora Sync."""

from __future__ import annotations

import argparse
import sys
import time

from ufora_sync.config import AppConfig
from ufora_sync.service import SyncService
from ufora_sync.sync import check_auth_status


def run_headless_service() -> None:
    """Run the background sync service in terminal / daemon mode."""
    print("🎓 Ufora Sync — Headless Background Service")
    service = SyncService()
    service.add_log_listener(lambda line: print(line))
    service.start()

    print("Daemon running. Press Ctrl+C to terminate.")
    try:
        while True:
            time.sleep(1)
    except KeyboardInterrupt:
        print("\nShutting down daemon…")
        service.stop()


def run_oneshot_sync() -> None:
    """Perform a single immediate sync pass and exit."""
    print("🎓 Ufora Sync — One-shot Sync")
    config = AppConfig.load()
    is_auth, msg = check_auth_status()
    if not is_auth:
        print(f"Error: {msg}. Run 'ufora login' first.")
        sys.exit(1)

    service = SyncService(config=config)
    service.add_log_listener(lambda line: print(line))
    service._do_sync_pass()


def run_desktop_app() -> None:
    """Run the combined System Tray and Settings GUI application."""
    from ufora_sync.app import UforaSyncApp
    from ufora_sync.tray import TrayApp

    config = AppConfig.load()
    service = SyncService(config=config)
    service.start()

    gui_window: UforaSyncApp | None = None

    def _open_gui():
        nonlocal gui_window
        if gui_window:
            gui_window.after(
                0,
                lambda: (
                    gui_window.deiconify(),
                    gui_window.lift(),
                    gui_window.focus_force(),
                ),
            )

    def _quit_all():
        nonlocal gui_window
        service.stop()
        if gui_window:
            gui_window.after(0, gui_window.destroy)

    # Initialize tray app in detached background thread
    tray = TrayApp(service=service, on_open_gui=_open_gui, on_quit=_quit_all)
    tray.run_detached()

    # Initialize GUI
    gui_window = UforaSyncApp(service=service, is_standalone=False)

    # Show window on initial launch if no courses are enabled yet
    if not config.enabled_courses:
        gui_window.deiconify()
    else:
        # User already configured courses: start minimized to menu bar / tray
        gui_window.withdraw()
        print("Ufora Sync started in background tray / menu bar.")

    try:
        gui_window.mainloop()
    finally:
        tray.stop()
        service.stop()


def main() -> None:
    parser = argparse.ArgumentParser(
        prog="ufora-sync",
        description="Cross-platform background sync service and tray application for UGent Ufora",
    )
    parser.add_argument(
        "command",
        nargs="?",
        default="tray",
        choices=["tray", "gui", "service", "sync", "login"],
        help=(
            "Command to run: 'tray' (default, menu bar app), 'gui' (settings window), "
            "'service' (headless daemon), 'sync' (one-off sync), 'login'"
        ),
    )

    args = parser.parse_args()

    if args.command == "service":
        run_headless_service()
    elif args.command == "sync":
        run_oneshot_sync()
    elif args.command == "login":
        import subprocess
        subprocess.run(["ufora", "login"])
    elif args.command == "gui":
        from ufora_sync.app import UforaSyncApp
        app = UforaSyncApp(is_standalone=True)
        app.mainloop()
    else:
        # Default: tray mode
        run_desktop_app()


if __name__ == "__main__":
    main()
