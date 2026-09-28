"""Entrypoint and CLI router for Ufora Sync."""

from __future__ import annotations

import argparse
import subprocess
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


_gui_process: subprocess.Popen | None = None


def open_settings_gui() -> None:
    """Launch or focus the Settings GUI in a dedicated process."""
    global _gui_process

    if _gui_process is not None and _gui_process.poll() is None:
        return
    _gui_process = subprocess.Popen([sys.executable, "-m", "ufora_sync", "gui"])



def run_desktop_app() -> None:
    """Run the system tray app on the main thread with background sync service."""
    from ufora_sync.tray import TrayApp

    config = AppConfig.load()
    service = SyncService(config=config)
    service.start()

    # If first run or no courses enabled yet, launch GUI so user can configure
    if not config.enabled_courses:
        open_settings_gui()
    else:
        print("Ufora Sync running in background menu bar / taskbar.")

    def _open_gui():
        open_settings_gui()

    def _quit_all():
        global _gui_process
        if _gui_process and _gui_process.poll() is None:
            _gui_process.terminate()
        service.stop()

    tray = TrayApp(service=service, on_open_gui=_open_gui, on_quit=_quit_all)
    try:
        tray.run()  # Native OS tray loop on the main thread
    finally:
        _quit_all()



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
