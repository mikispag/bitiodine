# Lessons Learned

- **Project & Package Naming Alignment**: When modernizing a repository whose project name is `<name>`, ensure the `Cargo.toml` package name (`name = "<name>"`), binary executable name, library crate import path, documentation, and tests use `<name>` instead of preserving outdated legacy suffixes like `-rust`.
