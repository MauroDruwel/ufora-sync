import { describe, expect, it } from "vitest";
import {
  invertCourses,
  isCourseEnabled,
  NO_COURSES_SELECTED,
  setCourseEnabled,
  setCoursesEnabled,
} from "./courseSelection";

const allCourses = ["course-a", "course-b", "course-c"];

describe("course selection", () => {
  it("keeps the legacy empty-list representation for all courses", () => {
    expect(isCourseEnabled([], "course-a")).toBe(true);
    expect(setCourseEnabled(["course-a", "course-b", "course-c"], allCourses, "course-c", true)).toEqual([]);
  });

  it("expands the all-courses sentinel before disabling one course", () => {
    const enabled = setCourseEnabled([], allCourses, "course-b", false);

    expect(enabled).toEqual(["course-a", "course-c"]);
    expect(isCourseEnabled(enabled, "course-b")).toBe(false);
  });

  it("can disable every course without accidentally selecting all", () => {
    const enabled = setCoursesEnabled([], allCourses, allCourses, false);

    expect(enabled).toEqual([NO_COURSES_SELECTED]);
    expect(allCourses.every((id) => !isCourseEnabled(enabled, id))).toBe(true);
  });

  it("enables a course from the all-disabled state", () => {
    expect(setCourseEnabled([NO_COURSES_SELECTED], allCourses, "course-b", true)).toEqual(["course-b"]);
  });

  it("changes only visible courses during bulk selection", () => {
    expect(setCoursesEnabled([], allCourses, ["course-a", "course-b"], false)).toEqual(["course-c"]);
    expect(setCoursesEnabled(["course-a"], allCourses, ["course-b"], true)).toEqual([
      "course-a",
      "course-b",
    ]);
  });

  it("inverts visible courses from both implicit-all and explicit selections", () => {
    expect(invertCourses([], allCourses, ["course-a", "course-c"])).toEqual(["course-b"]);
    expect(invertCourses(["course-a"], allCourses, ["course-a", "course-b"])).toEqual(["course-b"]);
  });
});
