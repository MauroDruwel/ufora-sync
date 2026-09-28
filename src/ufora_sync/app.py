"""Modern Settings & Course Selection GUI for Ufora Sync."""

from __future__ import annotations

import logging
import subprocess
import threading
from pathlib import Path
from tkinter import filedialog, messagebox
from typing import Any

import customtkinter as ctk

from ufora_sync.config import AppConfig
from ufora_sync.service import SyncService
from ufora_sync.sync import (
    ConflictStrategy,
    CourseInfo,
    check_auth_status,
    get_student_name,
    list_courses,
)
from ufora_sync.tray import open_folder_in_os

logger = logging.getLogger("ufora_sync.app")

# ---------------------------------------------------------------------------
# Theme & Palette (Modern Slate / Catppuccin Dark)
# ---------------------------------------------------------------------------

ctk.set_appearance_mode("dark")
ctk.set_default_color_theme("blue")

BG_MAIN = "#11111b"
BG_CARD = "#181825"
BG_CARD_ALT = "#1e1e2e"
BG_ITEM_HOVER = "#2a2a3e"
FG_TEXT = "#cdd6f4"
FG_MUTED = "#6c7086"
ACCENT = "#3b82f6"
ACCENT_HOVER = "#60a5fa"
ACCENT_GREEN = "#10b981"
ACCENT_RED = "#ef4444"
ACCENT_AMBER = "#f59e0b"

FONT_TITLE = ("Inter", 18, "bold")
FONT_HEADING = ("Inter", 14, "bold")
FONT_BODY = ("Inter", 12)
FONT_BOLD = ("Inter", 12, "bold")
FONT_SMALL = ("Inter", 11)
FONT_MONO = ("JetBrains Mono", 11)


class CourseCard(ctk.CTkFrame):
    """Card representing an enrolled course with an activation toggle."""

    def __init__(
        self,
        parent: Any,
        course: CourseInfo,
        is_enabled: bool,
        sync_dir: Path,
        on_toggle: Any,
    ) -> None:
        super().__init__(parent, fg_color=BG_CARD_ALT, corner_radius=8, height=52)
        self.pack_propagate(False)

        self.course = course
        self.sync_dir = sync_dir
        self.on_toggle = on_toggle

        self._switch_var = ctk.BooleanVar(value=is_enabled)

        # Switch toggle
        self._switch = ctk.CTkSwitch(
            self,
            text="",
            variable=self._switch_var,
            command=self._handle_toggle,
            width=42,
            switch_width=38,
            switch_height=20,
            progress_color=ACCENT,
        )
        self._switch.pack(side="left", padx=(12, 8), pady=12)

        # Course labels
        info_frame = ctk.CTkFrame(self, fg_color="transparent")
        info_frame.pack(side="left", fill="both", expand=True, padx=4, pady=6)

        display_name = course.name
        if len(display_name) > 52:
            display_name = display_name[:50] + "…"

        self._name_label = ctk.CTkLabel(
            info_frame,
            text=display_name,
            font=FONT_BOLD,
            text_color=FG_TEXT,
            anchor="w",
        )
        self._name_label.pack(fill="x", anchor="w")

        code_str = f"Code: {course.code}  •  ID: {course.id}"
        self._sub_label = ctk.CTkLabel(
            info_frame,
            text=code_str,
            font=FONT_SMALL,
            text_color=FG_MUTED,
            anchor="w",
        )
        self._sub_label.pack(fill="x", anchor="w")

        # Open folder action button
        self._folder_btn = ctk.CTkButton(
            self,
            text="📂 Folder",
            width=76,
            height=28,
            font=FONT_SMALL,
            fg_color=BG_ITEM_HOVER,
            hover_color="#3c3c5c",
            text_color=FG_TEXT,
            command=self._open_course_folder,
        )
        self._folder_btn.pack(side="right", padx=12, pady=12)

    def _handle_toggle(self) -> None:
        if self.on_toggle:
            self.on_toggle(self.course.id, self._switch_var.get())

    def _open_course_folder(self) -> None:
        folder = self.sync_dir / self.course.folder_name
        folder.mkdir(parents=True, exist_ok=True)
        open_folder_in_os(folder)

    def set_enabled(self, val: bool) -> None:
        self._switch_var.set(val)


class UforaSyncApp(ctk.CTk):
    """Main Settings and Management Window."""

    def __init__(
        self,
        service: SyncService | None = None,
        is_standalone: bool = False,
    ) -> None:
        super().__init__()

        self.service = service
        self.is_standalone = is_standalone
        self.config = AppConfig.load()

        self.title("Ufora Sync — Settings & Courses")
        self.geometry("920x640")
        self.minsize(800, 540)
        self.configure(fg_color=BG_MAIN)

        self.protocol("WM_DELETE_WINDOW", self._on_close)

        # State
        self._courses: list[CourseInfo] = []
        self._course_cards: dict[str, CourseCard] = {}
        self._status_var = ctk.StringVar(value="Connecting…")
        self._sync_dir_var = ctk.StringVar(value=self.config.sync_dir)
        self._interval_var = ctk.StringVar(value=f"{self.config.interval_minutes} minutes")
        self._strategy_var = ctk.StringVar(
            value=ConflictStrategy.LABELS.get(
                self.config.conflict_strategy, ConflictStrategy.DUPLICATE
            )
        )
        self._suffix_var = ctk.StringVar(value=self.config.duplicate_suffix)

        self._build_ui()
        self._bind_service_events()
        self._refresh_courses()

    def _bind_service_events(self) -> None:
        if self.service:
            self.service.add_status_listener(
                lambda s: self.after(0, self._update_service_status, s)
            )
            self.service.add_log_listener(lambda t: self.after(0, self._append_log, t))

    def _build_ui(self) -> None:
        # ── Top Bar ──────────────────────────────────────────────────
        top_bar = ctk.CTkFrame(self, fg_color=BG_CARD, height=64, corner_radius=0)
        top_bar.pack(side="top", fill="x")
        top_bar.pack_propagate(False)

        # Title
        title_box = ctk.CTkFrame(top_bar, fg_color="transparent")
        title_box.pack(side="left", padx=20, pady=10)

        ctk.CTkLabel(
            title_box,
            text="🎓 Ufora Sync",
            font=FONT_TITLE,
            text_color=FG_TEXT,
        ).pack(side="left")

        # User Badge (e.g. 👤 Mauro Druwel)
        self._user_badge = ctk.CTkLabel(
            top_bar,
            text="",
            font=FONT_BOLD,
            text_color=ACCENT_GREEN,
        )
        self._user_badge.pack(side="left", padx=(14, 4), pady=16)

        # Status text
        self._status_label = ctk.CTkLabel(
            top_bar,
            textvariable=self._status_var,
            font=FONT_SMALL,
            text_color=FG_MUTED,
        )
        self._status_label.pack(side="left", padx=4, pady=16)

        # Quick action buttons
        actions = ctk.CTkFrame(top_bar, fg_color="transparent")
        actions.pack(side="right", padx=16, pady=12)

        # Login button (ONLY shown when authentication is expired/missing)
        self._login_btn = ctk.CTkButton(
            actions,
            text="🔑 Sign In to UGent",
            width=140,
            height=32,
            font=FONT_BOLD,
            fg_color=ACCENT_AMBER,
            hover_color="#d97706",
            command=self._do_login,
        )

        self._sync_now_btn = ctk.CTkButton(
            actions,
            text="🔄 Sync Now",
            width=100,
            height=32,
            font=FONT_BOLD,
            fg_color=ACCENT,
            hover_color=ACCENT_HOVER,
            command=self._trigger_sync,
        )
        self._sync_now_btn.pack(side="left", padx=4)

        # ── Tab Navigation View ───────────────────────────────────────
        self._tabview = ctk.CTkTabview(
            self,
            fg_color=BG_MAIN,
            segmented_button_fg_color=BG_CARD,
            segmented_button_selected_color=ACCENT,
            segmented_button_selected_hover_color=ACCENT_HOVER,
            segmented_button_unselected_hover_color=BG_ITEM_HOVER,
            text_color=FG_TEXT,
        )
        self._tabview.pack(fill="both", expand=True, padx=16, pady=(8, 16))

        tab_courses = self._tabview.add("  📚 Courses  ")
        tab_settings = self._tabview.add("  ⚙️ Sync Settings  ")
        tab_logs = self._tabview.add("  📋 Activity Log  ")

        self._build_courses_tab(tab_courses)
        self._build_settings_tab(tab_settings)
        self._build_logs_tab(tab_logs)

    # ------------------------------------------------------------------
    # Courses Tab
    # ------------------------------------------------------------------

    def _build_courses_tab(self, parent: Any) -> None:
        toolbar = ctk.CTkFrame(parent, fg_color="transparent", height=38)
        toolbar.pack(fill="x", padx=4, pady=(2, 6))

        ctk.CTkLabel(
            toolbar,
            text="Choose which courses to keep synced to your PC:",
            font=FONT_BODY,
            text_color=FG_MUTED,
        ).pack(side="left")

        ctk.CTkButton(
            toolbar,
            text="Enable All",
            width=84,
            height=26,
            font=FONT_SMALL,
            fg_color=BG_CARD_ALT,
            hover_color=BG_ITEM_HOVER,
            command=lambda: self._set_all_courses(True),
        ).pack(side="right", padx=4)

        ctk.CTkButton(
            toolbar,
            text="Disable All",
            width=84,
            height=26,
            font=FONT_SMALL,
            fg_color=BG_CARD_ALT,
            hover_color=BG_ITEM_HOVER,
            command=lambda: self._set_all_courses(False),
        ).pack(side="right", padx=4)

        self._courses_scroll = ctk.CTkScrollableFrame(
            parent,
            fg_color="transparent",
            scrollbar_button_color="#2b2b3d",
        )
        self._courses_scroll.pack(fill="both", expand=True, padx=2, pady=4)

        self._courses_loading = ctk.CTkLabel(
            self._courses_scroll,
            text="Fetching enrolled courses from Ufora…",
            font=FONT_BODY,
            text_color=FG_MUTED,
        )
        self._courses_loading.pack(pady=40)

    # ------------------------------------------------------------------
    # Settings Tab
    # ------------------------------------------------------------------

    def _build_settings_tab(self, parent: Any) -> None:
        form = ctk.CTkScrollableFrame(parent, fg_color="transparent")
        form.pack(fill="both", expand=True, padx=12, pady=8)

        # 1. Sync Directory
        ctk.CTkLabel(
            form, text="Local Destination Folder", font=FONT_HEADING, text_color=FG_TEXT
        ).pack(anchor="w", pady=(0, 4))

        dir_box = ctk.CTkFrame(form, fg_color="transparent")
        dir_box.pack(fill="x", pady=(0, 18))

        ctk.CTkEntry(
            dir_box,
            textvariable=self._sync_dir_var,
            font=FONT_BODY,
            fg_color=BG_CARD_ALT,
            border_color="#313145",
            text_color=FG_TEXT,
            height=36,
        ).pack(side="left", fill="x", expand=True, padx=(0, 8))

        ctk.CTkButton(
            dir_box,
            text="Browse…",
            width=88,
            height=36,
            font=FONT_BODY,
            fg_color=BG_ITEM_HOVER,
            hover_color="#3c3c5c",
            command=self._pick_sync_folder,
        ).pack(side="left")

        # 2. Sync Frequency
        ctk.CTkLabel(
            form, text="Background Sync Frequency", font=FONT_HEADING, text_color=FG_TEXT
        ).pack(anchor="w", pady=(0, 4))

        interval_options = [
            "15 minutes",
            "30 minutes",
            "1 hour",
            "2 hours",
            "4 hours",
        ]
        self._interval_menu = ctk.CTkOptionMenu(
            form,
            values=interval_options,
            variable=self._interval_var,
            font=FONT_BODY,
            fg_color=BG_CARD_ALT,
            button_color=BG_ITEM_HOVER,
            text_color=FG_TEXT,
            dropdown_fg_color=BG_CARD_ALT,
            dropdown_text_color=FG_TEXT,
            height=34,
            width=220,
        )
        self._interval_menu.pack(anchor="w", pady=(0, 18))

        # 3. Conflict Resolution
        ctk.CTkLabel(
            form, text="When a File Has Local Edits", font=FONT_HEADING, text_color=FG_TEXT
        ).pack(anchor="w", pady=(0, 4))

        strategy_labels = list(ConflictStrategy.LABELS.values())
        self._strategy_menu = ctk.CTkOptionMenu(
            form,
            values=strategy_labels,
            variable=self._strategy_var,
            font=FONT_BODY,
            fg_color=BG_CARD_ALT,
            button_color=BG_ITEM_HOVER,
            text_color=FG_TEXT,
            dropdown_fg_color=BG_CARD_ALT,
            dropdown_text_color=FG_TEXT,
            height=34,
            width=420,
        )
        self._strategy_menu.pack(anchor="w", pady=(0, 12))

        # Suffix
        suffix_box = ctk.CTkFrame(form, fg_color="transparent")
        suffix_box.pack(anchor="w", pady=(0, 24))

        ctk.CTkLabel(
            suffix_box,
            text="Edited file suffix (for duplicate strategy):",
            font=FONT_BODY,
            text_color=FG_MUTED,
        ).pack(side="left", padx=(0, 8))

        ctk.CTkEntry(
            suffix_box,
            textvariable=self._suffix_var,
            font=FONT_BODY,
            width=100,
            height=30,
            fg_color=BG_CARD_ALT,
            border_color="#313145",
            text_color=FG_TEXT,
        ).pack(side="left")

        # Save Settings Action
        ctk.CTkButton(
            form,
            text="💾 Save Settings",
            width=140,
            height=36,
            font=FONT_BOLD,
            fg_color=ACCENT,
            hover_color=ACCENT_HOVER,
            command=self._save_settings,
        ).pack(anchor="w", pady=(0, 24))

        # 4. Account & Authentication Management
        ctk.CTkLabel(form, text="Account & Session", font=FONT_HEADING, text_color=FG_TEXT).pack(
            anchor="w", pady=(0, 4)
        )

        account_box = ctk.CTkFrame(form, fg_color=BG_CARD_ALT, corner_radius=8)
        account_box.pack(fill="x", pady=(0, 12))

        self._account_label = ctk.CTkLabel(
            account_box,
            text="Checking session…",
            font=FONT_BODY,
            text_color=FG_TEXT,
        )
        self._account_label.pack(side="left", padx=16, pady=12)

        ctk.CTkButton(
            account_box,
            text="Sign Out",
            width=90,
            height=28,
            font=FONT_SMALL,
            fg_color=BG_ITEM_HOVER,
            hover_color=ACCENT_RED,
            command=self._do_logout,
        ).pack(side="right", padx=12, pady=12)

        ctk.CTkButton(
            account_box,
            text="Refresh Session",
            width=120,
            height=28,
            font=FONT_SMALL,
            fg_color=BG_ITEM_HOVER,
            hover_color="#3c3c5c",
            command=self._do_login,
        ).pack(side="right", padx=4, pady=12)

    # ------------------------------------------------------------------
    # Logs Tab
    # ------------------------------------------------------------------

    def _build_logs_tab(self, parent: Any) -> None:
        toolbar = ctk.CTkFrame(parent, fg_color="transparent", height=32)
        toolbar.pack(fill="x", padx=4, pady=(2, 6))

        ctk.CTkLabel(
            toolbar,
            text="Live daemon & sync event stream:",
            font=FONT_SMALL,
            text_color=FG_MUTED,
        ).pack(side="left")

        ctk.CTkButton(
            toolbar,
            text="Clear Log",
            width=76,
            height=24,
            font=FONT_SMALL,
            fg_color=BG_CARD_ALT,
            hover_color=BG_ITEM_HOVER,
            command=self._clear_log,
        ).pack(side="right")

        self._log_text = ctk.CTkTextbox(
            parent,
            font=FONT_MONO,
            fg_color=BG_CARD,
            text_color=FG_TEXT,
            corner_radius=8,
            activate_scrollbars=True,
        )
        self._log_text.pack(fill="both", expand=True, padx=2, pady=4)

    # ------------------------------------------------------------------
    # Actions & Handlers
    # ------------------------------------------------------------------

    def _refresh_courses(self) -> None:
        is_auth, auth_msg = check_auth_status()
        student_name = get_student_name() if is_auth else ""

        if is_auth:
            self._login_btn.pack_forget()
            display_user = f"👤 {student_name}" if student_name else "👤 Logged In"
            self._user_badge.configure(text=display_user, text_color=ACCENT_GREEN)
            self._status_var.set("• Connected")
            self._status_label.configure(text_color=FG_MUTED)
            self._account_label.configure(text=f"Logged in as: {student_name or 'UGent Student'}")
            self._courses_loading.configure(text="Loading courses from Ufora…")
            threading.Thread(target=self._fetch_courses_worker, daemon=True).start()
        else:
            self._user_badge.configure(text="⚠ Session Expired", text_color=ACCENT_AMBER)
            self._status_var.set(f"• {auth_msg}")
            self._status_label.configure(text_color=ACCENT_AMBER)
            self._login_btn.pack(side="left", padx=4)
            self._account_label.configure(text="Session expired or not logged in.")
            self._courses_loading.configure(
                text=(
                    "Authentication needed.\n"
                    "Click 'Sign In to UGent' above to connect your account."
                )
            )

    def _fetch_courses_worker(self) -> None:
        try:
            courses = list_courses()
            self.after(0, self._render_courses, courses)
        except Exception as exc:
            self.after(0, self._render_courses_error, str(exc))

    def _render_courses(self, courses: list[CourseInfo]) -> None:
        self._courses = courses
        self._courses_loading.pack_forget()

        # Clean existing cards
        for card in self._course_cards.values():
            card.destroy()
        self._course_cards.clear()

        if not courses:
            self._courses_loading.configure(
                text="No enrolled courses found for this academic year."
            )
            self._courses_loading.pack(pady=40)
            return

        sync_dir = Path(self._sync_dir_var.get()).expanduser()

        for course in courses:
            is_enabled = self.config.is_course_enabled(course.id)
            card = CourseCard(
                self._courses_scroll,
                course=course,
                is_enabled=is_enabled,
                sync_dir=sync_dir,
                on_toggle=self._on_course_toggled,
            )
            card.pack(fill="x", padx=4, pady=3)
            self._course_cards[course.id] = card

    def _render_courses_error(self, err: str) -> None:
        self._courses_loading.configure(text=f"Error loading courses:\n{err}")
        self._courses_loading.pack(pady=40)

    def _on_course_toggled(self, course_id: str, enabled: bool) -> None:
        self.config.toggle_course(course_id, enabled)
        self.config.save()
        status_word = "ENABLED" if enabled else "DISABLED"
        self._append_log(f"Course {course_id} sync set to {status_word}")

    def _set_all_courses(self, enable: bool) -> None:
        for cid, card in self._course_cards.items():
            card.set_enabled(enable)
            self.config.toggle_course(cid, enable)
        self.config.save()
        status_word = "ENABLED" if enable else "DISABLED"
        self._append_log(f"All courses set to {status_word}")

    def _save_settings(self) -> None:
        self.config.sync_dir = self._sync_dir_var.get().strip()

        # Parse interval
        raw_int = self._interval_var.get()
        if "15" in raw_int:
            self.config.interval_minutes = 15
        elif "30" in raw_int:
            self.config.interval_minutes = 30
        elif "1 hour" in raw_int:
            self.config.interval_minutes = 60
        elif "2 hour" in raw_int:
            self.config.interval_minutes = 120
        elif "4 hour" in raw_int:
            self.config.interval_minutes = 240

        # Parse strategy
        selected_label = self._strategy_var.get()
        for key, label in ConflictStrategy.LABELS.items():
            if label == selected_label:
                self.config.conflict_strategy = key
                break

        self.config.duplicate_suffix = self._suffix_var.get().strip() or "_edited"
        self.config.save()
        self._append_log("Configuration saved successfully.")
        messagebox.showinfo("Ufora Sync", "Settings saved!")

    def _pick_sync_folder(self) -> None:
        chosen = filedialog.askdirectory(title="Select Ufora Sync Folder")
        if chosen:
            self._sync_dir_var.set(chosen)

    def _trigger_sync(self) -> None:
        self._sync_now_btn.configure(state="disabled", text="Syncing…")
        self._append_log("Starting sync pass…")

        active_service = self.service or SyncService(config=self.config)
        active_service.add_log_listener(lambda t: self.after(0, self._append_log, t))
        active_service.add_status_listener(lambda s: self.after(0, self._update_service_status, s))

        def _worker():
            try:
                active_service._do_sync_pass()
            finally:
                self.after(
                    0,
                    lambda: self._sync_now_btn.configure(state="normal", text="🔄 Sync Now"),
                )

        threading.Thread(target=_worker, daemon=True).start()

    def _update_service_status(self, status: str) -> None:
        self._status_var.set(f"• {status}")
        if "sync" in status.lower():
            self._status_label.configure(text_color=ACCENT)
            self._sync_now_btn.configure(state="disabled", text="Syncing…")
        elif "error" in status.lower() or "auth" in status.lower():
            self._status_label.configure(text_color=ACCENT_RED)
            self._sync_now_btn.configure(state="normal", text="🔄 Sync Now")
        else:
            self._status_label.configure(text_color=FG_MUTED)
            self._sync_now_btn.configure(state="normal", text="🔄 Sync Now")

    def _do_login(self) -> None:
        self._append_log("Opening UGent login in browser…")

        def _login_thread():
            try:
                subprocess.run(["ufora", "login"], check=False)
                self.after(0, self._refresh_courses)
            except Exception as e:
                self.after(0, self._append_log, f"Login error: {e}")

        threading.Thread(target=_login_thread, daemon=True).start()

    def _do_logout(self) -> None:
        confirm = messagebox.askyesno(
            "Sign Out",
            "Are you sure you want to sign out from UGent Ufora on this PC?",
        )
        if not confirm:
            return

        try:
            subprocess.run(["ufora", "logout"], check=False)
            profile_file = Path.home() / ".d2l" / "profile.json"
            if profile_file.exists():
                profile_file.unlink()
            self._append_log("Signed out from UGent Ufora.")
            self._refresh_courses()
        except Exception as e:
            self._append_log(f"Sign out error: {e}")

    def _append_log(self, text: str) -> None:
        self._log_text.insert("end", text + "\n")
        self._log_text.see("end")

    def _clear_log(self) -> None:
        self._log_text.delete("1.0", "end")

    def _on_close(self) -> None:
        self.destroy()
