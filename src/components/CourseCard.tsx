import { Course } from "../types";
import { Folder, ExternalLink } from "lucide-react";

interface CourseCardProps {
  course: Course;
  isEnabled: boolean;
  viewMode?: "list" | "grid";
  onToggle: (id: string, enabled: boolean) => void;
  onOpenFolder: (folderName: string) => void;
  onOpenUrl: (url: string) => void;
}

export const CourseCard = ({
  course,
  isEnabled,
  viewMode = "list",
  onToggle,
  onOpenFolder,
  onOpenUrl,
}: CourseCardProps) => {
  if (viewMode === "grid") {
    return (
      <div
        className={`p-4 rounded-2xl border transition duration-150 ease-[cubic-bezier(0.2,0,0,1)] flex flex-col justify-between group ${
          isEnabled
            ? "bg-[#f0f4f9] dark:bg-[#1e1f20] border-black/[0.06] dark:border-white/[0.06] hover:bg-[#e9eef6] dark:hover:bg-[#282a2c]"
            : "bg-[#f0f4f9]/50 dark:bg-[#1e1f20]/50 border-black/[0.04] dark:border-white/[0.04] opacity-50 hover:opacity-80"
        }`}
      >
        {/* Top: Code Badge + Material 3 Switch */}
        <div className="flex items-center justify-between mb-2.5">
          <span className="text-[11px] font-mono font-medium px-2 py-0.5 rounded-full bg-[#ffffff] dark:bg-[#282a2c] text-[#444746] dark:text-[#c4c7c5] border border-black/[0.08] dark:border-white/[0.08]">
            {course.code || "COURSE"}
          </span>
          <label className="relative inline-flex items-center cursor-pointer shrink-0">
            <input
              type="checkbox"
              checked={isEnabled}
              onChange={(e) => onToggle(course.id, e.target.checked)}
              className="sr-only peer"
            />
            <div className="w-9 h-5 bg-[#e9eef6] dark:bg-[#282a2c] border border-black/[0.12] dark:border-white/[0.12] peer-focus:outline-none rounded-full peer peer-checked:bg-[#1a73e8] dark:peer-checked:bg-[#a8c7fa] peer-checked:border-transparent peer-checked:after:translate-x-4 peer-checked:after:bg-[#ffffff] dark:peer-checked:after:bg-[#062e6f] after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-[#747775] dark:after:bg-[#8e918f] after:rounded-full after:h-4 after:w-4 after:transition-all after:duration-150"></div>
          </label>
        </div>

        {/* Middle: Course Name & ID */}
        <div className="flex-1 mb-3">
          <h3
            className="text-sm font-medium text-[#1f1f1f] dark:text-[#e3e3e3] line-clamp-2 leading-snug"
            title={course.name}
          >
            {course.name}
          </h3>
          <p className="text-[11px] text-[#747775] dark:text-[#8e918f] font-mono mt-1">
            ID: {course.id}
          </p>
        </div>

        {/* Bottom: Action buttons */}
        <div className="flex items-center justify-between pt-2.5 border-t border-black/[0.06] dark:border-white/[0.06]">
          <button
            onClick={() => onOpenFolder(course.folder_name)}
            className="flex items-center space-x-1.5 px-3 py-1 rounded-full text-xs font-medium bg-[#ffffff] dark:bg-[#282a2c] hover:bg-[#f0f4f9] dark:hover:bg-[#37393b] text-[#444746] dark:text-[#c4c7c5] hover:text-[#1f1f1f] dark:hover:text-[#e3e3e3] border border-black/[0.08] dark:border-white/[0.08] transition duration-150 ease-[cubic-bezier(0.2,0,0,1)] active:scale-[0.96]"
            title={`Open folder: ${course.folder_name}`}
          >
            <Folder className="w-3.5 h-3.5 text-[#1a73e8] dark:text-[#a8c7fa]" />
            <span>Folder</span>
          </button>

          {course.home_url && (
            <button
              onClick={() => onOpenUrl(course.home_url)}
              className="w-7 h-7 rounded-full flex items-center justify-center text-[#747775] dark:text-[#8e918f] hover:text-[#1a73e8] dark:hover:text-[#a8c7fa] hover:bg-[#ffffff] dark:hover:bg-[#282a2c] transition duration-150"
              title="Open course on Ufora"
            >
              <ExternalLink className="w-3.5 h-3.5" />
            </button>
          )}
        </div>
      </div>
    );
  }

  return (
    <div
      className={`px-4 py-3.5 rounded-2xl border transition duration-150 ease-[cubic-bezier(0.2,0,0,1)] flex items-center justify-between group ${
        isEnabled
          ? "bg-[#f0f4f9] dark:bg-[#1e1f20] border-black/[0.06] dark:border-white/[0.06] hover:bg-[#e9eef6] dark:hover:bg-[#282a2c]"
          : "bg-[#f0f4f9]/50 dark:bg-[#1e1f20]/50 border-black/[0.04] dark:border-white/[0.04] opacity-50 hover:opacity-80"
      }`}
    >
      <div className="flex items-center space-x-3.5 min-w-0 pr-4">
        {/* Material 3 Switch */}
        <label className="relative inline-flex items-center cursor-pointer shrink-0">
          <input
            type="checkbox"
            checked={isEnabled}
            onChange={(e) => onToggle(course.id, e.target.checked)}
            className="sr-only peer"
          />
          <div className="w-9 h-5 bg-[#e9eef6] dark:bg-[#282a2c] border border-black/[0.12] dark:border-white/[0.12] peer-focus:outline-none rounded-full peer peer-checked:bg-[#1a73e8] dark:peer-checked:bg-[#a8c7fa] peer-checked:border-transparent peer-checked:after:translate-x-4 peer-checked:after:bg-[#ffffff] dark:peer-checked:after:bg-[#062e6f] after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-[#747775] dark:after:bg-[#8e918f] after:rounded-full after:h-4 after:w-4 after:transition-all after:duration-150"></div>
        </label>

        {/* Course info */}
        <div className="min-w-0">
          <div className="flex items-center space-x-2">
            <h3 className="text-sm font-medium text-[#1f1f1f] dark:text-[#e3e3e3] truncate">
              {course.name}
            </h3>
            {course.home_url && (
              <button
                onClick={() => onOpenUrl(course.home_url)}
                className="opacity-0 group-hover:opacity-100 transition duration-150 text-[#747775] dark:text-[#8e918f] hover:text-[#1a73e8] dark:hover:text-[#a8c7fa]"
                title="Open course on Ufora"
              >
                <ExternalLink className="w-3.5 h-3.5" />
              </button>
            )}
          </div>
          <p className="text-xs text-[#747775] dark:text-[#8e918f] truncate font-sans">
            Code: <span className="font-mono text-[#444746] dark:text-[#c4c7c5]">{course.code}</span> • ID:{" "}
            <span className="font-mono text-[#444746] dark:text-[#c4c7c5]">{course.id}</span>
          </p>
        </div>
      </div>

      {/* Action button */}
      <div className="flex items-center space-x-2 shrink-0">
        <button
          onClick={() => onOpenFolder(course.folder_name)}
          className="flex items-center space-x-1.5 px-3 py-1 rounded-full text-xs font-medium bg-[#ffffff] dark:bg-[#282a2c] hover:bg-[#f0f4f9] dark:hover:bg-[#37393b] text-[#444746] dark:text-[#c4c7c5] hover:text-[#1f1f1f] dark:hover:text-[#e3e3e3] border border-black/[0.08] dark:border-white/[0.08] transition duration-150 ease-[cubic-bezier(0.2,0,0,1)] active:scale-[0.96]"
          title={`Open folder: ${course.folder_name}`}
        >
          <Folder className="w-3.5 h-3.5 text-[#1a73e8] dark:text-[#a8c7fa]" />
          <span>Folder</span>
        </button>
      </div>
    </div>
  );
};
