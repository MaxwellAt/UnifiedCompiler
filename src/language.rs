use crate::protocol::LanguagePreset;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct DetectedProject {
    pub language: LanguagePreset,
    pub entry_file: Option<String>,
    pub default_command: Vec<String>,
    pub hydration_commands: Vec<Vec<String>>,
}

pub fn detect_project(dir: &Path) -> DetectedProject {
    // 1. Rust Cargo project
    if dir.join("Cargo.toml").is_file() {
        return DetectedProject {
            language: LanguagePreset::Rust,
            entry_file: Some("Cargo.toml".to_string()),
            default_command: vec!["cargo".to_string(), "run".to_string(), "--release".to_string()],
            hydration_commands: vec![vec!["cargo".to_string(), "check".to_string()]],
        };
    }

    // 2. Python project
    if dir.join("pyproject.toml").is_file()
        || dir.join("requirements.txt").is_file()
        || dir.join("main.py").is_file()
    {
        let main_file = if dir.join("main.py").is_file() {
            "main.py"
        } else if dir.join("app.py").is_file() {
            "app.py"
        } else {
            "main.py"
        };

        let mut hydration = Vec::new();
        if dir.join("requirements.txt").is_file() {
            hydration.push(vec![
                "python".to_string(),
                "-m".to_string(),
                "pip".to_string(),
                "install".to_string(),
                "-r".to_string(),
                "requirements.txt".to_string(),
                "--quiet".to_string(),
            ]);
        }

        return DetectedProject {
            language: LanguagePreset::Python,
            entry_file: Some(main_file.to_string()),
            default_command: vec!["python".to_string(), main_file.to_string()],
            hydration_commands: hydration,
        };
    }

    // 3. Node.js project
    if dir.join("package.json").is_file() {
        return DetectedProject {
            language: LanguagePreset::Node,
            entry_file: Some("package.json".to_string()),
            default_command: vec!["npm".to_string(), "start".to_string()],
            hydration_commands: vec![vec![
                "npm".to_string(),
                "install".to_string(),
                "--prefer-offline".to_string(),
                "--no-audit".to_string(),
            ]],
        };
    }

    // 4. JVM / Java / Gradle / Maven project
    if dir.join("pom.xml").is_file() {
        return DetectedProject {
            language: LanguagePreset::Jvm,
            entry_file: Some("pom.xml".to_string()),
            default_command: vec!["mvn".to_string(), "clean".to_string(), "package".to_string()],
            hydration_commands: vec![],
        };
    }

    if dir.join("build.gradle").is_file() || dir.join("build.gradle.kts").is_file() {
        return DetectedProject {
            language: LanguagePreset::Jvm,
            entry_file: Some("build.gradle".to_string()),
            default_command: vec!["gradle".to_string(), "run".to_string()],
            hydration_commands: vec![],
        };
    }

    // Single Java file detection
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if name.ends_with(".java") {
                    let _class_name = name.trim_end_matches(".java").to_string();
                    return DetectedProject {
                        language: LanguagePreset::Jvm,
                        entry_file: Some(name.to_string()),
                        default_command: vec![
                            "java".to_string(),
                            name.to_string(),
                        ],
                        hydration_commands: vec![vec!["javac".to_string(), name.to_string()]],
                    };
                }
            }
        }
    }

    // Fallback Custom
    DetectedProject {
        language: LanguagePreset::Custom,
        entry_file: None,
        default_command: vec!["echo".to_string(), "No entrypoint detected".to_string()],
        hydration_commands: vec![],
    }
}
