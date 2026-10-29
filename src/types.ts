export interface Course {
  id: string;
  name: string;
  code: string;
  home_url: string;
  folder_name: string;
  is_course_offering?: boolean;
  academic_year?: number | null;
}

export interface AppConfig {
  sync_dir: string;
  interval_minutes: number;
  enabled_courses: string[];
  conflict_strategy: string;
  duplicate_suffix: string;
  sync_descriptions: boolean;
  sync_links: boolean;
  auto_start_tray: boolean;
  last_sync_time?: string | null;
  last_sync_status?: string | null;
}

export interface AuthStatus {
  is_authenticated: boolean;
  user_id?: string | null;
  message: string;
}

export interface SyncSummary {
  downloaded: number;
  up_to_date: number;
  errors: number;
}

export interface LogMessage {
  timestamp: string;
  message: string;
}

export interface UpdateInfo {
  current_version: string;
  latest_version: string;
  has_update: boolean;
  release_notes?: string | null;
  html_url: string;
  published_at?: string | null;
}
