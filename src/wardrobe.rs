//! Config-driven appearance, matching SARCharacterController's custom-skin rules.
use crate::{
    equipment,
    model::{Action, EquipmentData},
};
use rusty_spine::{Atlas, Physics, SkeletonData, Skin, c::*, controller::SkeletonController};
use serde::Deserialize;
use std::{collections::HashMap, ffi::CString, sync::Arc};

pub const CATEGORIES: [&str; 5] = ["Neck", "Clothes", "Glasses", "Hat", "Beard"];

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterItem {
    pub icon: String,
    #[serde(rename = "inventoryID")]
    pub inventory_id: String,
    pub spine_skin: String,
    pub spine_base_skin: String,
    #[serde(default)]
    pub needs_cut_hats: bool,
    #[serde(default)]
    pub custom_shia_headband: bool,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub icon: String,
    #[serde(rename = "inventoryID")]
    pub inventory_id: String,
    #[serde(default)]
    pub spine_parts: Vec<Part>,
    #[serde(default)]
    pub hide_hair1: bool,
    #[serde(default)]
    pub hide_hair2: bool,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Part {
    pub skin: String,
    pub slot: String,
    pub template: String,
    pub region: String,
    pub region_cut: Option<String>,
    pub include_front: Option<bool>,
}

pub struct Wardrobe {
    pub characters: Arc<Vec<CharacterItem>>,
    pub items: Arc<[Vec<Item>; 5]>,
    pub selected: [usize; 5],
    pub category: usize,
    pub current_character: usize,
    // Keep original attachments and their atlas alive until the custom skin is dropped.
    custom_skin: Option<Skin>,
    data: Arc<SkeletonData>,
    atlas: Arc<Atlas>,
    visible: Vec<(String, String, u8)>,
    hidden: Vec<String>,
    names: Arc<HashMap<String, HashMap<String, String>>>,
    pub language: String,
    pub available_languages: Arc<Vec<String>>,
    pub equipment_catalog: Arc<equipment::Catalog>,
    pub equipment: EquipmentData,
    pub action: Action,
    umbrella_atlas: Arc<Atlas>,
}

fn read<T: serde::de::DeserializeOwned>(name: &str) -> Result<T, String> {
    let path = format!("configs/_{name}.json");
    let bytes = std::fs::read(&path).map_err(|e| format!("Cannot read {path}: {e}"))?;
    serde_json::from_slice(&bytes).map_err(|e| format!("Invalid {path}: {e}"))
}

impl Wardrobe {
    pub fn display_name<'a>(&'a self, id: &'a str) -> &'a str {
        self.names
            .get(id)
            .and_then(|v| {
                v.get(&self.language)
                    .filter(|s| !s.is_empty())
                    .or_else(|| v.get("english"))
            })
            .map(String::as_str)
            .unwrap_or(id)
    }
    pub fn none_name(&self) -> &str {
        self.display_name("None")
    }
    /// Search only the selected translation; display fallback must not affect filtering.
    pub fn search_name<'a>(&'a self, id: &str) -> &'a str {
        self.names
            .get(id)
            .and_then(|names| names.get(&self.language))
            .map(String::as_str)
            .unwrap_or("")
    }
    pub fn load(data: Arc<SkeletonData>, atlas: Arc<Atlas>, skin: &str) -> Result<Self, String> {
        let configured: Vec<CharacterItem> = read("characters")?;
        let characters: Vec<_> = configured
            .into_iter()
            .filter(|item| {
                let valid = data.find_skin(&item.spine_skin).is_some()
                    && data.find_skin(&item.spine_base_skin).is_some();
                if !valid {
                    tracing::warn!(
                        "Skipping unavailable character {} ({})",
                        item.inventory_id,
                        item.spine_skin
                    );
                }
                valid
            })
            .collect();
        if characters.is_empty() {
            return Err("No configured characters match this skeleton".into());
        }
        let current_character = characters
            .iter()
            .position(|c| c.spine_skin == skin)
            .unwrap_or(0);
        let languages: HashMap<String, HashMap<String, String>> = read("languages")?;
        let mut available_languages: Vec<String> = languages
            .values()
            .flat_map(|entry| entry.keys().cloned())
            .collect();
        available_languages.sort();
        available_languages.dedup();
        let language = if available_languages.iter().any(|key| key == "english") {
            "english".to_owned()
        } else {
            available_languages
                .first()
                .cloned()
                .ok_or("Language configuration contains no language fields")?
        };
        let names = languages
            .into_iter()
            .filter_map(|(key, translations)| {
                key.strip_prefix("Item.")
                    .map(str::to_owned)
                    .map(|id| (id, translations))
            })
            .collect();
        let items: [Vec<Item>; 5] = [
            read("neck")?,
            read("clothes")?,
            read("glasses")?,
            read("hats")?,
            read("beards")?,
        ];
        if items
            .iter()
            .any(|items| items.is_empty() || !items[0].spine_parts.is_empty())
        {
            return Err("Every wardrobe category must start with an empty item".into());
        }
        tracing::info!(
            "Wardrobe loaded: {} characters; {} cosmetic items",
            characters.len(),
            items.iter().map(Vec::len).sum::<usize>()
        );
        let equipment_catalog = Arc::new(equipment::Catalog::load()?);
        let mut equipment = EquipmentData::default();
        equipment_catalog.initialize_loadout(&mut equipment);
        equipment.weapon = equipment_catalog
            .configured_melee(&equipment)
            .map(str::to_owned);
        equipment.emote = Some(equipment_catalog.configured_emote(&equipment).to_owned());
        Ok(Self {
            characters: Arc::new(characters),
            items: Arc::new(items),
            selected: [0; 5],
            category: 0,
            current_character,
            custom_skin: None,
            data,
            atlas,
            visible: vec![],
            hidden: vec![],
            names: Arc::new(names),
            language,
            available_languages: Arc::new(available_languages),
            equipment_catalog,
            equipment,
            action: Action::Idle,
            umbrella_atlas: Arc::new(
                Atlas::new_from_file("spineAssets/umbrellas/skeleton.atlas")
                    .map_err(|e| format!("Umbrella atlas: {e:?}"))?,
            ),
        })
    }

    /// Only immutable catalogs and original attachments are shared. Each entity
    /// owns its selected items, custom skin, visibility and animation mode.
    pub fn fork(&self) -> Self {
        Self {
            characters: self.characters.clone(),
            items: self.items.clone(),
            selected: self.selected,
            category: self.category,
            current_character: self.current_character,
            custom_skin: None,
            data: self.data.clone(),
            atlas: self.atlas.clone(),
            visible: vec![],
            hidden: vec![],
            names: self.names.clone(),
            language: self.language.clone(),
            available_languages: self.available_languages.clone(),
            equipment_catalog: self.equipment_catalog.clone(),
            equipment: self.equipment.clone(),
            action: self.action,
            umbrella_atlas: self.umbrella_atlas.clone(),
        }
    }

    pub fn item_name(&self, category: usize) -> &str {
        if self.selected[category] == 0 {
            self.none_name()
        } else {
            let id = &self.items[category][self.selected[category]].inventory_id;
            self.display_name(id)
        }
    }

    pub fn character_name(&self) -> &str {
        let id = &self.characters[self.current_character].inventory_id;
        self.display_name(id)
    }

    /// Build before applying: an invalid combination cannot damage the active appearance.
    pub fn apply(&mut self, controller: &mut SkeletonController) -> Result<(), String> {
        let character = &self.characters[self.current_character];
        let base = self
            .data
            .find_skin(&character.spine_skin)
            .ok_or("Missing character skin")?;
        let mut skin = Skin::new("wardrobe_custom");
        // AddSkin shares original character meshes, preserving their deform attachments.
        unsafe {
            skin.add_skin(&base);
        }
        let mut visible = vec![];
        let mut hidden = vec![];
        let mut parts = vec![];
        for (category, selected) in self.selected.iter().enumerate() {
            let item = &self.items[category][*selected];
            if item.hide_hair1 {
                hidden.push("hair".to_owned());
            }
            if item.hide_hair2 {
                hidden.push("hair2".to_owned());
            }
            parts.extend(item.spine_parts.iter().cloned().map(|p| (p, 0)));
        }
        parts.extend(
            self.equipment_catalog
                .weapon_parts(&self.equipment)
                .into_iter()
                .map(|p| (p, 1)),
        );
        if let Some(item) = self
            .equipment_catalog
            .umbrellas
            .iter()
            .find(|u| Some(u.inventory_id.as_str()) == self.equipment.umbrella.as_deref())
        {
            parts.extend(item.spine_parts.iter().cloned().map(|p| (p, 2)));
        }
        for (part, visibility) in &parts {
            let source = if part.skin == "#BASE_ANIMAL" {
                &character.spine_base_skin
            } else {
                &part.skin
            };
            let region = if character.needs_cut_hats && part.region_cut.is_some() {
                part.region_cut.as_deref().unwrap()
            } else if character.custom_shia_headband && part.region == "costumes/hat/headband_shia"
            {
                "costumes/hat/earband"
            } else {
                &part.region
            };
            let slot = self
                .data
                .find_slot(&part.slot)
                .ok_or_else(|| format!("Missing slot {}", part.slot))?;
            let slot_index = slot.index() as i32;
            let source = self
                .data
                .find_skin(source)
                .or_else(|| self.data.find_skin("default"))
                .ok_or("Missing template skin")?;
            let template_name = CString::new(part.template.as_str()).map_err(|e| e.to_string())?;
            let region_name = CString::new(region).map_err(|e| e.to_string())?;
            let alternate = region
                .strip_prefix("weapons/")
                .map(|key| format!("extras/{key}"));
            let main_region = self.atlas.find_region(region).or_else(|| {
                alternate
                    .as_deref()
                    .and_then(|name| self.atlas.find_region(name))
            });
            let external = main_region.is_none() && *visibility == 2;
            let atlas_region = main_region
                .or_else(|| {
                    if external {
                        self.umbrella_atlas
                            .find_region(region.rsplit('/').next().unwrap_or(region))
                    } else {
                        None
                    }
                })
                .ok_or_else(|| format!("Missing atlas region {region}"))?;
            // All pointers originate from data/atlas held by this wardrobe. Skin owns
            // copies through Spine's refcount; no original attachment is mutated.
            unsafe {
                let mut template =
                    spSkin_getAttachment(source.c_ptr(), slot_index, template_name.as_ptr());
                if template.is_null() {
                    if let Some(default) = self.data.find_skin("default") {
                        template = spSkin_getAttachment(
                            default.c_ptr(),
                            slot_index,
                            template_name.as_ptr(),
                        );
                    }
                }
                if template.is_null() {
                    return Err(format!(
                        "Missing template {} in {}",
                        part.template, part.slot
                    ));
                }
                remap(
                    &mut skin,
                    slot_index,
                    &region_name,
                    template,
                    atlas_region.c_ptr(),
                    external,
                )?;
                visible.push((part.slot.clone(), region.to_owned(), *visibility));
                let front = format!("{}_front", part.slot);
                if let Some(front_slot) = self.data.find_slot(&front) {
                    match part.include_front {
                        Some(true) => {
                            remap(
                                &mut skin,
                                front_slot.index() as i32,
                                &region_name,
                                template,
                                atlas_region.c_ptr(),
                                external,
                            )?;
                            visible.push((front, region.to_owned(), *visibility));
                        }
                        Some(false) => hidden.push(front),
                        None => {}
                    }
                }
            }
        }
        // Old skin remains alive while Spine swaps attachments. Retain the new skin
        // ourselves so set_skin does not deep-copy character-specific deform meshes.
        unsafe {
            controller.skeleton.set_skin_unchecked(&skin);
        }
        controller.skeleton.set_slots_to_setup_pose();
        self.custom_skin = Some(skin);
        self.visible = visible;
        self.hidden = hidden;
        controller.update(0.0, Physics::Reset);
        self.apply_visibility(controller);
        Ok(())
    }

    pub fn apply_visibility(&self, controller: &mut SkeletonController) {
        for (slot, attachment, group) in &self.visible {
            let show = match group {
                1 => self.action.weapons_visible(),
                2 => self.action.umbrella_visible(),
                _ => true,
            };
            if !show {
                controller.skeleton.set_attachment(slot, None);
                continue;
            }
            // Avoid resetting deform buffers when the attachment is already active.
            let active = controller
                .skeleton
                .find_slot(slot)
                .and_then(|s| s.attachment().map(|a| a.c_ptr()));
            let desired = controller
                .skeleton
                .get_attachment_for_slot_name(slot, attachment);
            if active != desired.as_ref().map(|a| a.c_ptr()) {
                controller.skeleton.set_attachment(slot, Some(attachment));
            }
        }
        for slot in &self.hidden {
            controller.skeleton.set_attachment(slot, None);
        }
    }
}

/// Mirrors Unity's Copy + Region + UpdateRegion, including the renderer's atlas pointer.
unsafe fn remap(
    skin: &mut Skin,
    slot: i32,
    name: &CString,
    template: *mut spAttachment,
    region: *mut spAtlasRegion,
    linked: bool,
) -> Result<(), String> {
    unsafe {
        let kind = (*template).type_0;
        if kind != SP_ATTACHMENT_REGION
            && kind != SP_ATTACHMENT_MESH
            && kind != SP_ATTACHMENT_LINKED_MESH
        {
            return Err(format!("Unsupported cosmetic template type {kind}"));
        }
        let copy = if linked && kind != SP_ATTACHMENT_REGION {
            spMeshAttachment_newLinkedMesh(template.cast()).cast()
        } else {
            spAttachment_copy(template)
        };
        if copy.is_null() {
            return Err("Failed to copy cosmetic attachment".into());
        }
        if kind == SP_ATTACHMENT_REGION {
            let attachment = copy.cast::<spRegionAttachment>();
            (*attachment).region = region.cast::<spTextureRegion>();
            (*attachment).rendererObject = region.cast();
            spRegionAttachment_updateRegion(attachment);
        } else {
            let attachment = copy.cast::<spMeshAttachment>();
            (*attachment).region = region.cast::<spTextureRegion>();
            (*attachment).rendererObject = region.cast();
            spMeshAttachment_updateRegion(attachment);
        }
        spSkin_setAttachment(skin.c_ptr(), slot, name.as_ptr(), copy);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusty_spine::{AnimationStateData, SkeletonBinary};

    #[test]
    fn all_weapon_skins_and_umbrellas_build_and_entities_are_independent() {
        let atlas = Arc::new(
            Atlas::new_from_file("spineAssets/characters/character_01/skeleton.atlas").unwrap(),
        );
        let data = Arc::new(
            SkeletonBinary::new(atlas.clone())
                .read_skeleton_data_file("spineAssets/characters/character_01/skeleton.skel.bytes")
                .unwrap(),
        );
        let state = Arc::new(AnimationStateData::new(data.clone()));
        let mut first = SkeletonController::new(data.clone(), state.clone());
        let mut wardrobe = Wardrobe::load(data.clone(), atlas, "Fox/-Fox").unwrap();
        wardrobe.apply(&mut first).unwrap();
        let mut second = SkeletonController::new(data.clone(), state);
        let mut other = wardrobe.fork();
        other.apply(&mut second).unwrap();
        let original: Vec<_> = second
            .skeleton
            .slots()
            .map(|s| s.attachment().map(|a| a.c_ptr()))
            .collect();
        let mut failures = vec![];
        let categories = wardrobe.equipment_catalog.categories.clone();
        for (category, c) in categories.iter().enumerate() {
            if !matches!(
                c.kind,
                equipment::CategoryKind::Weapon | equipment::CategoryKind::Umbrella
            ) {
                continue;
            }
            wardrobe.equipment = EquipmentData::default();
            for (index, entry) in c.entries.iter().enumerate() {
                wardrobe
                    .equipment_catalog
                    .choose(&mut wardrobe.equipment, category, index);
                // Configuration and held weapon are intentionally independent.
                wardrobe.equipment.weapon = entry
                    .weapon
                    .map(|weapon| wardrobe.equipment_catalog.weapons[weapon].id.clone());
                wardrobe.action = if matches!(c.kind, equipment::CategoryKind::Umbrella) {
                    Action::Umbrella
                } else {
                    Action::Idle
                };
                match wardrobe.apply(&mut first) {
                    Err(error) => failures.push(format!("{}/{}, {error}", c.label, entry.id)),
                    Ok(()) => {
                        first.update(0.05, Physics::Update);
                        wardrobe.apply_visibility(&mut first);
                        let mesh = first.combined_renderables();
                        assert!(
                            mesh.iter()
                                .all(|r| r.vertices.iter().flatten().all(|v| v.is_finite())),
                            "{}",
                            entry.id
                        );
                    }
                }
            }
        }
        assert_eq!(
            second
                .skeleton
                .slots()
                .map(|s| s.attachment().map(|a| a.c_ptr()))
                .collect::<Vec<_>>(),
            original
        );
        assert_eq!(other.equipment.weapon.as_deref(), Some("MeleeKatana"));
        assert!(Arc::ptr_eq(&wardrobe.items, &other.items));
        assert!(
            failures.is_empty(),
            "{} equipment failures:\n{}",
            failures.len(),
            failures.join("\n")
        );
        drop(first);
        drop(second);
    }

    #[test]
    fn configured_outfits_and_characters_rebuild_without_residue() {
        let atlas = Arc::new(
            Atlas::new_from_file("spineAssets/characters/character_01/skeleton.atlas").unwrap(),
        );
        let data = Arc::new(
            SkeletonBinary::new(atlas.clone())
                .read_skeleton_data_file("spineAssets/characters/character_01/skeleton.skel.bytes")
                .unwrap(),
        );
        let mut wardrobe = Wardrobe::load(data.clone(), atlas, "Fox/-Fox").unwrap();
        let mut controller =
            SkeletonController::new(data.clone(), Arc::new(AnimationStateData::new(data)));
        wardrobe.apply(&mut controller).unwrap();
        let initial_character = wardrobe.current_character;
        let initial_visible = wardrobe.visible.clone();
        let initial_hidden = wardrobe.hidden.clone();
        let initial_attachments: Vec<_> = controller
            .skeleton
            .slots()
            .map(|slot| slot.attachment().map(|a| a.c_ptr()))
            .collect();
        let mut failures = vec![];
        for category in 0..5 {
            for index in 0..wardrobe.items[category].len() {
                wardrobe.selected[category] = index;
                if let Err(error) = wardrobe.apply(&mut controller) {
                    failures.push(format!(
                        "{}: {error}",
                        wardrobe.items[category][index].inventory_id
                    ));
                } else {
                    controller.update(1.0 / 60.0, Physics::Update);
                    wardrobe.apply_visibility(&mut controller);
                    let _ = controller.combined_renderables();
                }
            }
            wardrobe.selected[category] = 0;
        }
        // Verify #BASE_ANIMAL templates and character flags across the catalog.
        wardrobe.selected = [1; 5];
        for index in 0..wardrobe.characters.len() {
            wardrobe.current_character = index;
            if let Err(error) = wardrobe.apply(&mut controller) {
                failures.push(format!(
                    "{}: {error}",
                    wardrobe.characters[index].inventory_id
                ));
            }
        }
        wardrobe.selected = [0; 5];
        wardrobe.current_character = initial_character;
        wardrobe.apply(&mut controller).unwrap();
        assert_eq!(wardrobe.visible, initial_visible);
        assert_eq!(wardrobe.hidden, initial_hidden);
        let cleared_attachments: Vec<_> = controller
            .skeleton
            .slots()
            .map(|slot| slot.attachment().map(|a| a.c_ptr()))
            .collect();
        assert_eq!(
            initial_attachments, cleared_attachments,
            "Clearing the outfit must restore the original character attachments"
        );
        assert!(
            failures.is_empty(),
            "{} invalid combinations:\n{}",
            failures.len(),
            failures.join("\n")
        );
        // Drop the skeleton before its custom skin, matching Character's field order.
        drop(controller);
    }
}
