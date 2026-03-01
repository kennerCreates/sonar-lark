use std::fs;
use std::path::Path;

use super::types::*;

pub fn discover_existing_courses() -> Vec<CourseEntry> {
    discover_existing_courses_in(Path::new("assets/courses"))
}

pub fn discover_existing_courses_in(courses_dir: &Path) -> Vec<CourseEntry> {
    let mut courses = Vec::new();
    if let Ok(entries) = fs::read_dir(courses_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("ron")
                && let Some(name) = path.file_stem().and_then(|s| s.to_str())
            {
                let display_name = name.trim_end_matches(".course").to_string();
                courses.push(CourseEntry {
                    display_name,
                    path: path.to_string_lossy().to_string(),
                });
            }
        }
    }
    courses.sort_by(|a, b| a.display_name.cmp(&b.display_name));
    courses
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn discover_empty_directory() {
        let dir = tempfile::tempdir().unwrap();
        let courses = discover_existing_courses_in(dir.path());
        assert!(courses.is_empty());
    }

    #[test]
    fn discover_nonexistent_directory() {
        let courses = discover_existing_courses_in(Path::new("no_such_dir_xyz"));
        assert!(courses.is_empty());
    }

    #[test]
    fn discover_filters_ron_only() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.course.ron"), "()").unwrap();
        fs::write(dir.path().join("b.txt"), "not a course").unwrap();
        fs::write(dir.path().join("c.json"), "{}").unwrap();

        let courses = discover_existing_courses_in(dir.path());
        assert_eq!(courses.len(), 1);
        assert_eq!(courses[0].display_name, "a");
    }

    #[test]
    fn discover_strips_course_suffix() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("my_track.course.ron"), "()").unwrap();

        let courses = discover_existing_courses_in(dir.path());
        assert_eq!(courses[0].display_name, "my_track");
    }

    #[test]
    fn discover_plain_ron_keeps_name() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("simple.ron"), "()").unwrap();

        let courses = discover_existing_courses_in(dir.path());
        assert_eq!(courses[0].display_name, "simple");
    }

    #[test]
    fn discover_results_sorted_alphabetically() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("charlie.course.ron"), "()").unwrap();
        fs::write(dir.path().join("alpha.course.ron"), "()").unwrap();
        fs::write(dir.path().join("bravo.course.ron"), "()").unwrap();

        let courses = discover_existing_courses_in(dir.path());
        let names: Vec<_> = courses.iter().map(|c| c.display_name.as_str()).collect();
        assert_eq!(names, vec!["alpha", "bravo", "charlie"]);
    }

    #[test]
    fn discover_stores_full_path() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("test.course.ron"), "()").unwrap();

        let courses = discover_existing_courses_in(dir.path());
        assert!(courses[0].path.contains("test.course.ron"));
    }
}
