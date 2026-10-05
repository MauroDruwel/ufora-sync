import { Course } from "../types";
import { Folder, ExternalLink } from "lucide-react";

interface CourseCardProps {
  course: Course;
  isEnabled: boolean;
  onToggle: (id: string, enabled: boolean) => void;
  onOpenFolder: (folderName: string) => void;
  onOpenUrl: (url: string) => void;
}

export const CourseCard = ({
  course,
  isEnabled,
  onToggle,
  onOpenFolder,
  onOpenUrl,
}: CourseCardProps) => {
  return (
    <div
      className={`p-3.5 rounded-xl border transition-all duration-200 flex items-center justify-between group ${
        isEnabled
          ? "bg-[#182234]/90 border-slate-700/80 shadow-sm hover:border-ugent-blue/50"
          : "bg-slate-900/40 border-slate-800/60 opacity-60 hover:opacity-80"
      }`}
    >
      <div className="flex items-center space-x-3.5 min-w-0 pr-4">
        {/* Toggle switch */}
        <label className="relative inline-flex items-center cursor-pointer shrink-0">
          <input
            type="checkbox"
            checked={isEnabled}
            onChange={(e) => onToggle(course.id, e.target.checked)}
            className="sr-only peer"
          />
          <div className="w-9 h-5 bg-slate-700 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-slate-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-ugent-blue"></div>
        </label>

        {/* Course info */}
        <div className="min-w-0">
          <div className="flex items-center space-x-2">
            <h3 className="text-sm font-semibold text-slate-100 truncate group-hover:text-white transition">
              {course.name}
            </h3>
            {course.home_url && (
              <button
                onClick={() => onOpenUrl(course.home_url)}
                className="opacity-0 group-hover:opacity-100 transition text-slate-400 hover:text-ugent-blue"
                title="Open on Ufora"
              >
                <ExternalLink className="w-3.5 h-3.5" />
              </button>
            )}
          </div>
          <p className="text-xs text-slate-400 truncate">
            Code: <span className="font-mono text-slate-300">{course.code}</span> • ID: <span className="font-mono text-slate-300">{course.id}</span>
          </p>
        </div>
      </div>

      {/* Action buttons */}
      <div className="flex items-center space-x-2 shrink-0">
        <button
          onClick={() => onOpenFolder(course.folder_name)}
          className="flex items-center space-x-1.5 px-2.5 py-1.5 rounded-lg text-xs font-medium bg-slate-800 hover:bg-slate-700 text-slate-300 border border-slate-700/60 transition active:scale-95 shadow-sm"
          title={`Open folder: ${course.folder_name}`}
        >
          <Folder className="w-3.5 h-3.5 text-ugent-blue" />
          <span>Folder</span>
        </button>
      </div>
    </div>
  );
};
