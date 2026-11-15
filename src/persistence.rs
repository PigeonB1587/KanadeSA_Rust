//! Two independent JSON documents with paired revision checking and crash recovery.
use crate::model::{CharacterFile, SceneFile, validate_pair};
use std::{fs, io::Write, path::Path};

pub fn new_id() -> String {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let count = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    format!("{time:x}-{:x}-{count:x}", std::process::id())
}

fn read_pair(directory: &Path, suffix: &str) -> Result<(CharacterFile, SceneFile), String> {
    let read = |name: &str| {
        fs::read(directory.join(format!("{name}.json{suffix}"))).map_err(|e| e.to_string())
    };
    let characters = serde_json::from_slice(&read("characters")?).map_err(|e| e.to_string())?;
    let scene = serde_json::from_slice(&read("scene")?).map_err(|e| e.to_string())?;
    validate_pair(&characters, &scene)?;
    Ok((characters, scene))
}

pub fn load(directory: &Path) -> Result<(CharacterFile, SceneFile), String> {
    match read_pair(directory, "") {
        Ok(pair) => Ok(pair),
        Err(error) => read_pair(directory, ".previous")
            .map_err(|backup| format!("Cannot load save: {error}; previous save: {backup}")),
    }
}

fn write_synced(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = fs::File::create(path).map_err(|e| e.to_string())?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())
}

pub fn save(directory: &Path, characters: &CharacterFile, scene: &SceneFile) -> Result<(), String> {
    validate_pair(characters, scene)?;
    let character_bytes = serde_json::to_vec(characters).map_err(|e| e.to_string())?;
    let scene_bytes = serde_json::to_vec(scene).map_err(|e| e.to_string())?;
    fs::create_dir_all(directory).map_err(|e| e.to_string())?;
    // Stage and sync both files before replacing either member. Keep a complete
    // previous pair; a crash between renames can never load a mixed generation.
    write_synced(&directory.join("characters.json.pending"), &character_bytes)?;
    write_synced(&directory.join("scene.json.pending"), &scene_bytes)?;
    if let Ok((previous_characters, previous_scene)) = load(directory) {
        write_synced(
            &directory.join("characters.json.previous.pending"),
            &serde_json::to_vec(&previous_characters).map_err(|e| e.to_string())?,
        )?;
        write_synced(
            &directory.join("scene.json.previous.pending"),
            &serde_json::to_vec(&previous_scene).map_err(|e| e.to_string())?,
        )?;
        for name in ["characters", "scene"] {
            fs::rename(
                directory.join(format!("{name}.json.previous.pending")),
                directory.join(format!("{name}.json.previous")),
            )
            .map_err(|e| e.to_string())?;
        }
        sync_directory(directory)?;
    }
    for name in ["characters", "scene"] {
        fs::rename(
            directory.join(format!("{name}.json.pending")),
            directory.join(format!("{name}.json")),
        )
        .map_err(|e| e.to_string())?;
    }
    sync_directory(directory)?;
    let restored = read_pair(directory, "")?;
    if restored.0 != *characters || restored.1 != *scene {
        return Err("Save read-back mismatch".into());
    }
    Ok(())
}

fn sync_directory(directory: &Path) -> Result<(), String> {
    #[cfg(unix)]
    fs::File::open(directory)
        .and_then(|f| f.sync_all())
        .map_err(|e| e.to_string())?;
    #[cfg(not(unix))]
    let _ = directory;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;

    fn pair() -> (CharacterFile, SceneFile) {
        let id = new_id();
        (
            CharacterFile {
                format_version: FORMAT_VERSION,
                revision: "1".into(),
                characters: vec![CharacterData {
                    id: id.clone(),
                    animal: "CharFox".into(),
                    outfits: std::array::from_fn(|i| format!("None-{i}")),
                    equipment: EquipmentData::default(),
                    level: 55,
                    animation: AnimationData::preset(Action::Emote),
                }],
            },
            SceneFile {
                format_version: FORMAT_VERSION,
                revision: "1".into(),
                background: "Booth/Welcome/WelcomeBG".into(),
                watermark: Watermark::Right,
                transparent: false,
                selected_character: Some(id.clone()),
                placements: vec![Placement {
                    character_id: id,
                    ..Default::default()
                }],
            },
        )
    }
    #[test]
    fn separate_files_round_trip_and_recover_an_interrupted_commit() {
        let dir = std::env::temp_dir().join(format!("kanade-save-{}", new_id()));
        let (mut c, mut s) = pair();
        c.characters[0].equipment = EquipmentData {
            weapon: Some("GunPistol".into()),
            emote: Some("EmoteWave".into()),
            gun_skins: [("GunAK".into(), "grey".into())].into(),
            loadout: Some(LoadoutData {
                melee: Some("MeleeKatana".into()),
                emote: "EmoteDanceBasic".into(),
            }),
            ..Default::default()
        };
        c.characters[0].animation.pet_animation = Some(PetAnimationData {
            clip: "mini-quadruped/idle".into(),
            time: 0.73,
        });
        save(&dir, &c, &s).unwrap();
        assert_eq!(load(&dir).unwrap(), (c.clone(), s.clone()));
        let scene_json: serde_json::Value =
            serde_json::from_slice(&fs::read(dir.join("scene.json")).unwrap()).unwrap();
        assert!(scene_json.get("characters").is_none());
        assert!(scene_json["placements"][0].get("equipment").is_none());
        c.revision = "2".into();
        s.revision = "2".into();
        c.characters[0].level = 999;
        save(&dir, &c, &s).unwrap();
        // Simulate the first current file being committed while the second is old.
        fs::copy(dir.join("scene.json.previous"), dir.join("scene.json")).unwrap();
        let recovered = load(&dir).unwrap();
        assert_eq!(recovered.0.revision, "1");
        assert_eq!(recovered.0.characters[0].level, 55);
        save(&dir, &c, &s).unwrap();
        assert_eq!(load(&dir).unwrap(), (c, s));
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn rejects_dangling_references_duplicate_ids_and_nonfinite_values() {
        let (mut c, mut s) = pair();
        s.placements[0].character_id = "missing".into();
        assert!(validate_pair(&c, &s).is_err());
        s.placements[0].character_id = c.characters[0].id.clone();
        c.characters.push(c.characters[0].clone());
        assert!(validate_pair(&c, &s).is_err());
        c.characters.pop();
        c.characters[0].animation.pet_animation = Some(PetAnimationData {
            clip: "mini-quadruped/idle".into(),
            time: -1.0,
        });
        assert!(validate_pair(&c, &s).is_err());
        c.characters[0].animation.pet_animation = None;
        s.placements[0].height = f32::NAN;
        assert!(validate_pair(&c, &s).is_err());
    }
}
