//! Named, isolated creative documents; user names never become filesystem paths.
use serde::{Deserialize, Serialize};
use std::{
    io::Write,
    path::{Path, PathBuf},
};

pub const ROOT: &str = "projects";
pub const FILE_EXTENSION: &str = ".kanade-project.json";
pub const RATIOS: [&str; 8] = ["16:9", "16:10", "4:3", "3:2", "1:1", "4:5", "3:4", "9:16"];

/// A complete editable 1.0 document. Media exports and installed assets are
/// deliberately separate; no source directory or source project ID is trusted.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectFile {
    pub format: String,
    pub format_version: u32,
    pub name: String,
    pub canvas_ratio: String,
    pub characters: crate::model::CharacterFile,
    pub scene: crate::model::SceneFile,
}

impl ProjectFile {
    pub fn new(
        project: &Project,
        characters: crate::model::CharacterFile,
        scene: crate::model::SceneFile,
    ) -> Self {
        Self {
            format: "kanade-project".into(),
            format_version: 1,
            name: project.name.clone(),
            canvas_ratio: project.canvas_ratio.clone(),
            characters,
            scene,
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.format != "kanade-project" || self.format_version != 1 {
            return Err("Expected a Kanade project 1.0 file".into());
        }
        clean_name(&self.name)?;
        if !RATIOS.contains(&self.canvas_ratio.as_str()) {
            return Err("Invalid project canvas ratio".into());
        }
        crate::model::validate_pair(&self.characters, &self.scene)?;
        if self.characters.characters.len() != self.scene.placements.len() {
            return Err("Every project character must have a scene placement".into());
        }
        Ok(())
    }
    pub fn read(path: &Path) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("Cannot read project file: {e}"))?;
        let file: Self =
            serde_json::from_slice(&bytes).map_err(|e| format!("Invalid project file: {e}"))?;
        file.validate()?;
        Ok(file)
    }
    pub fn write_new(&self, path: &Path) -> Result<(), String> {
        self.validate()?;
        let bytes = serde_json::to_vec(self).map_err(|e| e.to_string())?;
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        // create_new also closes the race with another export to the same path.
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::AlreadyExists {
                    "That file already exists; choose a different export path".to_owned()
                } else {
                    e.to_string()
                }
            })?;
        let result = file
            .write_all(&bytes)
            .and_then(|_| file.write_all(b"\n"))
            .and_then(|_| file.sync_all());
        drop(file);
        if let Err(error) = result {
            // Only remove the new file owned by this failed export.
            let _ = std::fs::remove_file(path);
            return Err(error.to_string());
        }
        Ok(())
    }
}

pub fn export_path(path: &Path) -> Result<PathBuf, String> {
    if path.as_os_str().is_empty() || path.file_name().is_none() || path.is_dir() {
        return Err("Enter a project file path, including its filename".into());
    }
    if path.as_os_str().to_string_lossy().ends_with(FILE_EXTENSION) {
        Ok(path.to_path_buf())
    } else {
        let mut path = path.as_os_str().to_os_string();
        path.push(FILE_EXTENSION);
        Ok(PathBuf::from(path))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Project {
    pub format_version: u32,
    pub id: String,
    pub name: String,
    pub canvas_ratio: String,
    pub updated_at: String,
    pub character_count: usize,
}

pub fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
}

pub fn clean_name(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() || name.chars().any(char::is_control) {
        return Err("Enter a project name without control characters".into());
    }
    if name.chars().count() > 80 {
        return Err("Project names can contain up to 80 characters".into());
    }
    Ok(name.to_owned())
}

impl Project {
    pub fn new(name: &str) -> Result<Self, String> {
        Ok(Self {
            format_version: 1,
            id: crate::persistence::new_id(),
            name: clean_name(name)?,
            canvas_ratio: "16:9".into(),
            updated_at: String::new(),
            character_count: 1,
        })
    }
    pub fn directory(&self) -> PathBuf {
        Path::new(ROOT).join(&self.id)
    }
    pub fn default_export_path(&self) -> PathBuf {
        Path::new("exports").join(format!(
            "{}-{}{FILE_EXTENSION}",
            self.id,
            chrono::Local::now().format("%Y%m%d-%H%M%S-%3f")
        ))
    }
    pub fn read(id: &str) -> Result<Self, String> {
        if !valid_id(id) {
            return Err("Invalid project ID".into());
        }
        let project: Self = serde_json::from_slice(
            &std::fs::read(Path::new(ROOT).join(id).join("project.json"))
                .map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        if project.format_version != 1
            || project.id != id
            || !RATIOS.contains(&project.canvas_ratio.as_str())
        {
            return Err("Unsupported or inconsistent project metadata".into());
        }
        clean_name(&project.name)?;
        Ok(project)
    }
    pub fn write(&self) -> Result<(), String> {
        if !valid_id(&self.id)
            || self.format_version != 1
            || !RATIOS.contains(&self.canvas_ratio.as_str())
        {
            return Err("Invalid project metadata".into());
        }
        clean_name(&self.name)?;
        let directory = self.directory();
        std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        write_atomic(
            &directory.join("project.json"),
            &serde_json::to_vec(self).map_err(|e| e.to_string())?,
        )
    }
}

pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let pending = path.with_extension(format!(
        "{}.pending",
        path.extension().unwrap_or_default().to_string_lossy()
    ));
    let mut file = std::fs::File::create(&pending).map_err(|e| e.to_string())?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())?;
    drop(file);
    std::fs::rename(pending, path).map_err(|e| e.to_string())
}

pub fn list() -> Result<Vec<Project>, String> {
    let entries = match std::fs::read_dir(ROOT) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.to_string()),
    };
    let mut projects = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        if !entry.file_type().map_err(|e| e.to_string())?.is_dir() {
            continue;
        }
        let id = entry.file_name().to_string_lossy().into_owned();
        if !entry.path().join("project.json").is_file() {
            continue;
        }
        match Project::read(&id) {
            Ok(project) => projects.push(project),
            Err(error) => tracing::warn!("Skipping project {id}: {error}"),
        }
    }
    projects.sort_by(|a, b| {
        b.updated_at
            .cmp(&a.updated_at)
            .then_with(|| a.name.cmp(&b.name))
    });
    Ok(projects)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn document() -> ProjectFile {
        use crate::model::*;
        ProjectFile::new(
            &Project::new("合影 / scene").unwrap(),
            CharacterFile {
                format_version: FORMAT_VERSION,
                revision: "paired".into(),
                characters: vec![CharacterData {
                    id: "actor".into(),
                    animal: "CharFox".into(),
                    outfits: std::array::from_fn(|i| format!("None-{i}")),
                    equipment: EquipmentData::default(),
                    level: 55,
                    animation: AnimationData::preset(Action::Idle),
                }],
            },
            SceneFile {
                format_version: FORMAT_VERSION,
                revision: "paired".into(),
                background: "Booth/Welcome/WelcomeBG".into(),
                watermark: Watermark::Right,
                transparent: false,
                selected_character: Some("actor".into()),
                placements: vec![Placement {
                    character_id: "actor".into(),
                    ..Default::default()
                }],
            },
        )
    }

    #[test]
    fn project_file_retains_creation_and_every_supported_ratio() {
        let mut file = document();
        for ratio in RATIOS {
            file.canvas_ratio = ratio.into();
            let bytes = serde_json::to_vec(&file).unwrap();
            let restored: ProjectFile = serde_json::from_slice(&bytes).unwrap();
            restored.validate().unwrap();
            assert_eq!(restored.name, file.name);
            assert_eq!(restored.canvas_ratio, ratio);
            assert_eq!(restored.characters, file.characters);
            assert_eq!(restored.scene, file.scene);
        }
        file.canvas_ratio = "invalid".into();
        assert!(file.validate().is_err());
        file.canvas_ratio = "4:5".into();
        file.format_version = 2;
        assert!(file.validate().is_err());
        file.format_version = 1;
        file.scene.placements[0].character_id = "missing".into();
        assert!(file.validate().is_err());
        file.scene.placements.clear();
        file.scene.selected_character = None;
        assert!(file.validate().is_err());
    }

    #[test]
    fn project_export_never_overwrites_an_existing_file() {
        let path = std::env::temp_dir().join(format!(
            "kanade-project-{}{FILE_EXTENSION}",
            crate::persistence::new_id()
        ));
        let mut file = document();
        file.canvas_ratio = "9:16".into();
        file.write_new(&path).unwrap();
        let original = std::fs::read(&path).unwrap();
        file.name = "Changed".into();
        assert!(file.write_new(&path).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), original);
        let restored = ProjectFile::read(&path).unwrap();
        assert_eq!(restored.canvas_ratio, "9:16");
        assert_ne!(restored.name, file.name);
        std::fs::remove_file(path).unwrap();
        assert!(export_path(Path::new("")).is_err());
        assert_eq!(
            export_path(Path::new("scene")).unwrap(),
            PathBuf::from("scene.kanade-project.json")
        );
    }

    #[test]
    fn names_are_independent_of_safe_unique_directory_ids() {
        let a = Project::new("  樱花 / 合影 ..  ").unwrap();
        let b = Project::new("樱花 / 合影 ..").unwrap();
        assert_eq!(a.name, "樱花 / 合影 ..");
        assert_ne!(a.id, b.id);
        assert!(valid_id(&a.id));
        for id in ["../escape", "/absolute", "a/b", "", "..", "a\\b"] {
            assert!(!valid_id(id));
        }
        assert!(Project::new("   ").is_err());
        assert!(Project::new("bad\nname").is_err());
    }
}
