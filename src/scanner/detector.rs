use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProjectType {
    Rust,
    JavaScript,
    TypeScript,
    Python,
    Go,
    Java,
    CSharp,
    Cpp,
    Ruby,
    Php,
    Git,
    Unknown,
}

impl ProjectType {
    pub fn name(&self) -> &'static str {
        match self {
            ProjectType::Rust => "Rust",
            ProjectType::JavaScript => "JavaScript",
            ProjectType::TypeScript => "TypeScript",
            ProjectType::Python => "Python",
            ProjectType::Go => "Go",
            ProjectType::Java => "Java",
            ProjectType::CSharp => "C#",
            ProjectType::Cpp => "C/C++",
            ProjectType::Ruby => "Ruby",
            ProjectType::Php => "PHP",
            ProjectType::Git => "Git Repo",
            ProjectType::Unknown => "Unknown",
        }
    }
}

const PROJECT_MARKERS: &[(&str, ProjectType)] = &[
    ("Cargo.toml", ProjectType::Rust),
    ("package.json", ProjectType::JavaScript),
    ("tsconfig.json", ProjectType::TypeScript),
    ("deno.json", ProjectType::TypeScript),
    ("requirements.txt", ProjectType::Python),
    ("setup.py", ProjectType::Python),
    ("pyproject.toml", ProjectType::Python),
    ("Pipfile", ProjectType::Python),
    ("go.mod", ProjectType::Go),
    ("pom.xml", ProjectType::Java),
    ("build.gradle", ProjectType::Java),
    ("build.gradle.kts", ProjectType::Java),
    (".csproj", ProjectType::CSharp),
    (".sln", ProjectType::CSharp),
    ("CMakeLists.txt", ProjectType::Cpp),
    ("Makefile", ProjectType::Cpp),
    ("Gemfile", ProjectType::Ruby),
    ("composer.json", ProjectType::Php),
    (".git", ProjectType::Git),
];

pub fn detect_project_type(path: &Path) -> Option<ProjectType> {
    for (marker, project_type) in PROJECT_MARKERS {
        let marker_path = path.join(marker);
        if marker_path.exists() {
            return Some(*project_type);
        }
    }
    None
}

pub fn is_project_directory(path: &Path) -> bool {
    detect_project_type(path).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_detect_rust_project() {
        let temp_dir = TempDir::new().unwrap();
        let cargo_toml = temp_dir.path().join("Cargo.toml");
        fs::write(&cargo_toml, "[package]").unwrap();

        assert_eq!(
            detect_project_type(temp_dir.path()),
            Some(ProjectType::Rust)
        );
    }

    #[test]
    fn test_detect_javascript_project() {
        let temp_dir = TempDir::new().unwrap();
        let package_json = temp_dir.path().join("package.json");
        fs::write(&package_json, "{}").unwrap();

        assert_eq!(
            detect_project_type(temp_dir.path()),
            Some(ProjectType::JavaScript)
        );
    }

    #[test]
    fn test_detect_no_project() {
        let temp_dir = TempDir::new().unwrap();
        assert_eq!(detect_project_type(temp_dir.path()), None);
    }

    #[test]
    fn test_is_project_directory() {
        let temp_dir = TempDir::new().unwrap();
        assert!(!is_project_directory(temp_dir.path()));

        let cargo_toml = temp_dir.path().join("Cargo.toml");
        fs::write(&cargo_toml, "[package]").unwrap();
        assert!(is_project_directory(temp_dir.path()));
    }
}
