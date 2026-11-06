/**
 * The backend uses an empty enabled_courses array as the legacy representation
 * for "all courses". A reserved value is needed for the opposite state because
 * an empty array cannot also represent "no courses".
 */
export const NO_COURSES_SELECTED = "__ufora_sync_no_courses_selected__";

export function isCourseEnabled(enabledCourses: string[], courseId: string): boolean {
  return enabledCourses.length === 0 || enabledCourses.includes(courseId);
}

function effectiveSelection(enabledCourses: string[], knownCourseIds: string[]): Set<string> {
  if (enabledCourses.length === 0) {
    return new Set(knownCourseIds);
  }

  return new Set(enabledCourses.filter((id) => id !== NO_COURSES_SELECTED));
}

function serializeSelection(selection: Set<string>, knownCourseIds: string[]): string[] {
  const uniqueKnownIds = Array.from(new Set(knownCourseIds));

  if (uniqueKnownIds.length > 0 && uniqueKnownIds.every((id) => selection.has(id))) {
    return [];
  }

  if (selection.size === 0) {
    return [NO_COURSES_SELECTED];
  }

  return Array.from(selection);
}

export function setCourseEnabled(
  enabledCourses: string[],
  knownCourseIds: string[],
  courseId: string,
  enabled: boolean,
): string[] {
  const selection = effectiveSelection(enabledCourses, knownCourseIds);

  if (enabled) {
    selection.add(courseId);
  } else {
    selection.delete(courseId);
  }

  return serializeSelection(selection, knownCourseIds);
}

export function setCoursesEnabled(
  enabledCourses: string[],
  knownCourseIds: string[],
  targetCourseIds: string[],
  enabled: boolean,
): string[] {
  const selection = effectiveSelection(enabledCourses, knownCourseIds);

  for (const courseId of targetCourseIds) {
    if (enabled) {
      selection.add(courseId);
    } else {
      selection.delete(courseId);
    }
  }

  return serializeSelection(selection, knownCourseIds);
}

export function invertCourses(
  enabledCourses: string[],
  knownCourseIds: string[],
  targetCourseIds: string[],
): string[] {
  const selection = effectiveSelection(enabledCourses, knownCourseIds);

  for (const courseId of targetCourseIds) {
    if (selection.has(courseId)) {
      selection.delete(courseId);
    } else {
      selection.add(courseId);
    }
  }

  return serializeSelection(selection, knownCourseIds);
}
