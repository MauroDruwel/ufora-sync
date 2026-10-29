export function formatRelativeSyncTime(
  isoString?: string | null,
  referenceTime: number = Date.now()
): string {
  if (!isoString) return "Ready to sync";
  const date = new Date(isoString);
  const time = date.getTime();
  if (isNaN(time)) return "Ready to sync";

  const diffSec = Math.max(0, Math.floor((referenceTime - time) / 1000));
  if (diffSec < 45) {
    return "Synced just now";
  }
  const diffMin = Math.floor(diffSec / 60);
  if (diffMin < 60) {
    return `Synced ${diffMin} min ago`;
  }
  const diffHours = Math.floor(diffMin / 60);
  const hours = date.getHours().toString().padStart(2, "0");
  const minutes = date.getMinutes().toString().padStart(2, "0");

  if (diffHours < 12) {
    return `Last synced ${hours}:${minutes}`;
  }

  const monthNames = [
    "Jan",
    "Feb",
    "Mar",
    "Apr",
    "May",
    "Jun",
    "Jul",
    "Aug",
    "Sep",
    "Oct",
    "Nov",
    "Dec",
  ];
  return `Last synced ${monthNames[date.getMonth()]} ${date.getDate()}, ${hours}:${minutes}`;
}
