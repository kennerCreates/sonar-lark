use std::fs;
use std::path::Path;

use super::loader::load_course_from_file;

pub struct CourseEntry {
    pub name: String,
    pub path: String,
    pub gate_count: usize,
}

pub fn discover_courses() -> Vec<CourseEntry> {
    discover_courses_in(Path::new("assets/courses"))
}

pub fn discover_courses_in(courses_dir: &Path) -> Vec<CourseEntry> {
    let mut courses = Vec::new();

    if let Ok(entries) = fs::read_dir(courses_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("ron")
                && let Some(name) = path.file_stem().and_then(|s| s.to_str())
            {
                let display_name = name.trim_end_matches(".course").to_string();
                let gate_count = load_course_from_file(&path)
                    .map(|c| {
                        c.instances
                            .iter()
                            .filter(|i| i.gate_order.is_some())
                            .count()
                    })
                    .unwrap_or(0);
                courses.push(CourseEntry {
                    name: display_name,
                    path: path.to_string_lossy().to_string(),
                    gate_count,
                });
            }
        }
    }

    courses.sort_by(|a, b| a.name.cmp(&b.name));
    courses
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_course_ron() -> &'static str {
        r#"(name: "empty", instances: [])"#
    }

    fn course_ron_with_gates(gate_count: usize) -> String {
        let mut instances = String::new();
        for i in 0..gate_count {
            if i > 0 {
                instances.push_str(", ");
            }
            instances.push_str(&format!(
                r#"(obstacle_id: ("gate"), translation: (0.0, 0.0, {z}.0), rotation: (0.0, 0.0, 0.0, 1.0), scale: (1.0, 1.0, 1.0), gate_order: Some({i}), gate_forward_flipped: false)"#,
                z = -(i as i32) * 20,
                i = i,
            ));
        }
        format!(r#"(name: "test", instances: [{instances}])"#)
    }

    #[test]
    fn discover_empty_directory() {
        let dir = tempfile::tempdir().unwrap();
        let courses = discover_courses_in(dir.path());
        assert!(courses.is_empty());
    }

    #[test]
    fn discover_nonexistent_directory() {
        let courses = discover_courses_in(Path::new("this/does/not/exist"));
        assert!(courses.is_empty());
    }

    #[test]
    fn discover_filters_ron_only() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("track.course.ron"), empty_course_ron()).unwrap();
        fs::write(dir.path().join("readme.txt"), "ignore me").unwrap();
        fs::write(dir.path().join("notes.md"), "ignore me too").unwrap();

        let courses = discover_courses_in(dir.path());
        assert_eq!(courses.len(), 1);
        assert_eq!(courses[0].name, "track");
    }

    #[test]
    fn discover_strips_course_suffix() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("mountain.course.ron"), empty_course_ron()).unwrap();
        fs::write(dir.path().join("simple.ron"), empty_course_ron()).unwrap();

        let courses = discover_courses_in(dir.path());
        assert_eq!(courses.len(), 2);
        assert_eq!(courses[0].name, "mountain");
        assert_eq!(courses[1].name, "simple");
    }

    #[test]
    fn discover_results_sorted_alphabetically() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("zebra.course.ron"), empty_course_ron()).unwrap();
        fs::write(dir.path().join("alpha.course.ron"), empty_course_ron()).unwrap();
        fs::write(dir.path().join("middle.course.ron"), empty_course_ron()).unwrap();

        let courses = discover_courses_in(dir.path());
        let names: Vec<&str> = courses.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, vec!["alpha", "middle", "zebra"]);
    }

    #[test]
    fn discover_stores_full_path() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.course.ron");
        fs::write(&file_path, empty_course_ron()).unwrap();

        let courses = discover_courses_in(dir.path());
        assert_eq!(courses.len(), 1);
        assert!(courses[0].path.contains("test.course.ron"));
    }

    #[test]
    fn discover_counts_gates() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("no_gates.course.ron"), empty_course_ron()).unwrap();
        fs::write(
            dir.path().join("two_gates.course.ron"),
            course_ron_with_gates(2),
        )
        .unwrap();
        fs::write(
            dir.path().join("five_gates.course.ron"),
            course_ron_with_gates(5),
        )
        .unwrap();

        let courses = discover_courses_in(dir.path());
        assert_eq!(courses.len(), 3);
        // Sorted: five_gates, no_gates, two_gates
        assert_eq!(courses[0].name, "five_gates");
        assert_eq!(courses[0].gate_count, 5);
        assert_eq!(courses[1].name, "no_gates");
        assert_eq!(courses[1].gate_count, 0);
        assert_eq!(courses[2].name, "two_gates");
        assert_eq!(courses[2].gate_count, 2);
    }

    #[test]
    fn discover_invalid_ron_gets_zero_gates() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("bad.course.ron"), "not valid ron").unwrap();

        let courses = discover_courses_in(dir.path());
        assert_eq!(courses.len(), 1);
        assert_eq!(courses[0].name, "bad");
        assert_eq!(courses[0].gate_count, 0);
    }
}
