"""Entrypoint and CLI router for Ufora Sync."""

from __future__ import annotations

import argparse
import subprocess
import sys
import time

from ufora_sync.config import AppConfig
from ufora_sync.service import SyncService
from ufora_sync.sync import ensure_authenticated


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
    is_auth, msg = ensure_authenticated()
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
    service.add_log_listener(lambda line: print(line))
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
    from ufora_sync import __version__

    parser = argparse.ArgumentParser(
        prog="ufora-sync",
        description=(
            "Unofficial background sync service and tray application for UGent Ufora "
            "(not affiliated with UGent or D2L)"
        ),
    )
    parser.add_argument(
        "--version",
        "-v",
        action="version",
        version=f"ufora-sync {__version__}",
    )
    parser.add_argument(
        "command",
        nargs="?",
        default="tray",
        choices=["tray", "gui", "service", "sync", "login", "autostart"],
        help=(
            "Command to run: 'tray' (default, menu bar app), 'gui' (settings window), "
            "'service' (headless daemon), 'sync' (one-off sync), 'login', 'autostart'"
        ),
    )
    parser.add_argument(
        "autostart_action",
        nargs="?",
        default="status",
        choices=["status", "enable", "disable"],
        help="Action for 'autostart' command: 'status' (default), 'enable', 'disable'",
    )

    args = parser.parse_args()

    if args.command == "service":
        run_headless_service()
    elif args.command == "sync":
        run_oneshot_sync()
    elif args.command == "login":
        import subprocess

        subprocess.run(["ufora", "login"])
    elif args.command == "autostart":
        from ufora_sync.autostart import disable_autostart, enable_autostart, is_autostart_enabled

        action = args.autostart_action
        if action == "enable":
            ok, msg = enable_autostart()
            print(msg if ok else f"Error: {msg}")
            sys.exit(0 if ok else 1)
        elif action == "disable":
            ok, msg = disable_autostart()
            print(msg if ok else f"Error: {msg}")
            sys.exit(0 if ok else 1)
        else:
            enabled = is_autostart_enabled()
            print(f"Ufora Sync auto-start on login: {'ENABLED' if enabled else 'DISABLED'}")
    elif args.command == "gui":
        from ufora_sync.app import UforaSyncApp

        app = UforaSyncApp(is_standalone=True)
        app.mainloop()
    else:
        # Default: tray mode
        run_desktop_app()


if __name__ == "__main__":
    main()
