# Testing Conventions

- Tests live in `#[cfg(test)] mod tests` at the bottom of each source file (idiomatic Rust).
- Use `tempfile` crate (dev-dependency) for filesystem tests — never write to the real `assets/` directory.
- Test pure logic and serialization (file I/O, data structures, discovery). ECS systems are tested manually.
- When adding file I/O functions, provide a parameterized version that accepts a `&Path` so tests can use temp directories (e.g., `discover_courses_in(path)` vs `discover_courses()`).
