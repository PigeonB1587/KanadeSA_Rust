//! Equipment catalog and flat weapon categories. Selections use inventory IDs/skin names.
use crate::{
    model::{EquipmentData, LoadoutData},
    wardrobe::{Item, Part},
};
use serde::Deserialize;

#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Weapon {
    #[serde(rename = "inventoryID")]
    pub id: String,
    pub icon: String,
    #[serde(default)]
    pub use_cosmetics_anim_track: bool,
    pub weapon_class: String,
    #[serde(default)]
    pub spine_attachment_key: String,
    pub spine_aim_anim_key: Option<String>,
    pub spine_atk_anim_key: Option<String>,
    pub spine_atk_alternate_anim_key: Option<String>,
    pub reload_spine_anim_key: Option<String>,
    #[serde(default)]
    pub shot_interval: f32,
    #[serde(default)]
    pub spine_slots_attachments_regions: Vec<[String; 3]>,
    #[serde(default)]
    pub skin_names: Vec<String>,
    #[serde(default)]
    pub skin_names_alt: Vec<String>,
    #[serde(default)]
    pub skin_names2: Vec<String>,
    pub grenade_info: Option<Grenade>,
}
#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Grenade {
    pub spine_attachment: Option<String>,
}
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pet {
    #[serde(rename = "inventoryID")]
    pub id: String,
    pub icon: String,
    #[serde(default)]
    pub skin: String,
    #[serde(default)]
    pub anim_type: u8,
}
#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Emote {
    #[serde(rename = "inventoryID")]
    pub id: String,
    #[serde(default)]
    pub icon: String,
    pub emote_spine_key: String,
    pub emote_spine_key_initial: Option<String>,
    pub emote_spine_key_walk: Option<String>,
    pub pet_anim: Option<String>,
}
#[derive(Clone, Debug)]
pub struct Entry {
    pub id: String,
    pub icon: String,
    pub weapon: Option<usize>,
    pub skin: Option<usize>,
}
#[derive(Clone, Debug)]
pub enum CategoryKind {
    Weapon,
    Umbrella,
    Pet,
    Emote,
}
#[derive(Clone, Debug)]
pub struct Category {
    pub label: String,
    pub kind: CategoryKind,
    pub entries: Vec<Entry>,
}
pub struct Catalog {
    pub weapons: Vec<Weapon>,
    pub umbrellas: Vec<Item>,
    pub pets: Vec<Pet>,
    pub emotes: Vec<Emote>,
    pub categories: Vec<Category>,
}
fn read<T: serde::de::DeserializeOwned>(name: &str) -> Result<T, String> {
    crate::assets::read_config(name)
}
impl Catalog {
    pub fn load() -> Result<Self, String> {
        let weapons: Vec<Weapon> = read("weapons")?;
        let umbrellas: Vec<Item> = read("umbrellas")?;
        let pets: Vec<Pet> = read("pets")?;
        let emotes: Vec<Emote> = read("emotes")?;
        let mut categories = vec![];
        let mut entries = vec![];
        entries.extend(
            weapons
                .iter()
                .enumerate()
                .filter(|(_, w)| w.weapon_class == "Melee")
                .map(|(i, w)| Entry {
                    id: w.id.clone(),
                    icon: w.icon.clone(),
                    weapon: Some(i),
                    skin: None,
                }),
        );
        categories.push(Category {
            label: "Weapon-Melee".into(),
            kind: CategoryKind::Weapon,
            entries,
        });
        for (i, w) in weapons
            .iter()
            .enumerate()
            .filter(|(_, w)| w.weapon_class == "Gun")
        {
            let entries = w
                .skin_names
                .iter()
                .enumerate()
                .map(|(skin, name)| Entry {
                    id: format!("{}_{}", w.id, name),
                    icon: format!("{}{name}", w.icon),
                    weapon: Some(i),
                    skin: Some(skin),
                })
                .collect();
            let label = match w.id.as_str() {
                "GunAK" => "AK47",
                "GunUzi" => "UZI",
                "GunDualUzi" => "DualUZI",
                id => id.strip_prefix("Gun").unwrap_or(id),
            };
            categories.push(Category {
                label: format!("Weapon-{label}"),
                kind: CategoryKind::Weapon,
                entries,
            });
        }
        for (kind, label, entries) in [
            (
                CategoryKind::Umbrella,
                "Umbrellas",
                umbrellas
                    .iter()
                    .map(|i| Entry {
                        id: i.inventory_id.clone(),
                        icon: i.icon.clone(),
                        weapon: None,
                        skin: None,
                    })
                    .collect::<Vec<_>>(),
            ),
            (
                CategoryKind::Pet,
                "Pets",
                pets.iter()
                    .map(|i| Entry {
                        id: i.id.clone(),
                        icon: i.icon.clone(),
                        weapon: None,
                        skin: None,
                    })
                    .collect(),
            ),
            (
                CategoryKind::Emote,
                "Emotes",
                emotes
                    .iter()
                    .map(|i| Entry {
                        id: i.id.clone(),
                        icon: i.icon.clone(),
                        weapon: None,
                        skin: None,
                    })
                    .collect(),
            ),
        ] {
            categories.push(Category {
                label: label.into(),
                kind,
                entries,
            });
        }
        Ok(Self {
            weapons,
            umbrellas,
            pets,
            emotes,
            categories,
        })
    }
    /// Mirrors Unity SkinGenerator.GetEnableSparkle, including its category exclusions.
    pub fn has_sparkle(&self, category: usize, index: usize) -> bool {
        let category = &self.categories[category];
        match category.kind {
            CategoryKind::Weapon => category.entries[index]
                .weapon
                .is_some_and(|weapon| self.weapons[weapon].use_cosmetics_anim_track),
            CategoryKind::Umbrella => self.umbrellas[index].use_cosmetics_anim_track,
            CategoryKind::Pet | CategoryKind::Emote => false,
        }
    }
    pub fn choose(&self, equipment: &mut EquipmentData, category: usize, index: usize) {
        let Some(c) = self.categories.get(category) else {
            return;
        };
        let Some(entry) = c.entries.get(index) else {
            return;
        };
        self.initialize_loadout(equipment);
        let value = (entry.id != "None").then(|| entry.id.clone());
        match c.kind {
            CategoryKind::Weapon => {
                if let (Some(w), Some(s)) = (entry.weapon, entry.skin) {
                    equipment.gun_skins.insert(
                        self.weapons[w].id.clone(),
                        self.weapons[w].skin_names[s].clone(),
                    );
                } else {
                    equipment.loadout.as_mut().unwrap().melee =
                        entry.weapon.map(|i| self.weapons[i].id.clone());
                }
            }
            CategoryKind::Umbrella => equipment.umbrella = value,
            CategoryKind::Pet => equipment.pet = value,
            CategoryKind::Emote => equipment.loadout.as_mut().unwrap().emote = entry.id.clone(),
        }
    }
    pub fn selected(&self, equipment: &EquipmentData, category: usize) -> usize {
        let c = &self.categories[category];
        c.entries
            .iter()
            .enumerate()
            .find(|(_, e)| match c.kind {
                CategoryKind::Weapon => e.weapon.is_some_and(|w| {
                    (self.weapons[w].weapon_class == "Gun"
                        || self.configured_melee(equipment) == Some(self.weapons[w].id.as_str()))
                        && e.skin
                            .is_none_or(|s| self.skin_index(equipment, &self.weapons[w]) == s)
                }),
                CategoryKind::Umbrella => equipment.umbrella.as_ref() == Some(&e.id),
                CategoryKind::Pet => equipment.pet.as_ref() == Some(&e.id),
                CategoryKind::Emote => self.configured_emote(equipment) == e.id,
            })
            .map_or(0, |(i, _)| i)
    }
    pub fn initialize_loadout(&self, e: &mut EquipmentData) {
        if e.umbrella.is_none() {
            e.umbrella = self.umbrellas.first().map(|item| item.inventory_id.clone());
        }
        if e.pet.is_none() {
            e.pet = self.pets.iter().next().map(|pet| pet.id.clone());
        }
        if e.loadout.is_none() {
            e.loadout = Some(LoadoutData {
                melee: self
                    .weapon(e)
                    .filter(|w| w.weapon_class == "Melee")
                    .or_else(|| self.weapons.iter().find(|w| w.weapon_class == "Melee"))
                    .map(|w| w.id.clone()),
                emote: self
                    .emote(e)
                    .or_else(|| self.emotes.first())
                    .map(|e| e.id.clone())
                    .unwrap_or_default(),
            });
        }
    }
    pub fn configured_melee<'a>(&'a self, e: &'a EquipmentData) -> Option<&'a str> {
        e.loadout.as_ref().map_or_else(
            || {
                self.weapons
                    .iter()
                    .find(|w| w.weapon_class == "Melee")
                    .map(|w| w.id.as_str())
            },
            |l| l.melee.as_deref(),
        )
    }
    pub fn configured_emote<'a>(&'a self, e: &'a EquipmentData) -> &'a str {
        e.loadout
            .as_ref()
            .map(|l| l.emote.as_str())
            .or(e.emote.as_deref())
            .or_else(|| self.emotes.first().map(|e| e.id.as_str()))
            .unwrap_or("")
    }
    /// One booth entry per gun, using that character's configured skin.
    pub fn booth_weapons(&self, e: &EquipmentData) -> Vec<Entry> {
        let mut entries = vec![];
        if let Some(w) = self
            .weapons
            .iter()
            .find(|w| Some(w.id.as_str()) == self.configured_melee(e))
        {
            entries.push(Entry {
                id: w.id.clone(),
                icon: w.icon.clone(),
                weapon: self.weapons.iter().position(|item| item.id == w.id),
                skin: None,
            });
        }
        entries.extend(
            self.weapons
                .iter()
                .enumerate()
                .filter(|(_, w)| w.weapon_class == "Gun")
                .map(|(index, w)| {
                    let skin = self.skin_index(e, w);
                    Entry {
                        id: w.id.clone(),
                        icon: self.weapon_icon(e, w),
                        weapon: Some(index),
                        skin: Some(skin),
                    }
                }),
        );
        entries
    }
    pub fn weapon_icon(&self, e: &EquipmentData, w: &Weapon) -> String {
        if w.weapon_class == "Gun" {
            format!(
                "{}{}",
                w.icon,
                w.skin_names
                    .get(self.skin_index(e, w))
                    .map_or("", String::as_str)
            )
        } else {
            w.icon.clone()
        }
    }
    pub fn weapon<'a>(&'a self, e: &EquipmentData) -> Option<&'a Weapon> {
        self.weapons
            .iter()
            .find(|w| Some(w.id.as_str()) == e.weapon.as_deref())
    }
    pub fn emote<'a>(&'a self, e: &EquipmentData) -> Option<&'a Emote> {
        self.emotes
            .iter()
            .find(|w| Some(w.id.as_str()) == e.emote.as_deref())
    }
    pub fn pet<'a>(&'a self, e: &EquipmentData) -> Option<&'a Pet> {
        self.pets
            .iter()
            .find(|w| !w.skin.is_empty() && Some(w.id.as_str()) == e.pet.as_deref())
    }
    pub fn skin_index(&self, e: &EquipmentData, w: &Weapon) -> usize {
        e.gun_skins
            .get(&w.id)
            .and_then(|s| w.skin_names.iter().position(|name| name == s))
            .unwrap_or(0)
    }
    pub fn weapon_parts(&self, e: &EquipmentData) -> Vec<Part> {
        let Some(w) = self.weapon(e) else {
            return vec![];
        };
        let make = |slot: &str, template: &str, region: String| Part {
            skin: "default".into(),
            slot: slot.into(),
            template: template.into(),
            region,
            region_cut: None,
            include_front: Some(false),
        };
        match w.weapon_class.as_str() {
            "Melee" if w.spine_attachment_key.is_empty() => vec![],
            "Melee" => vec![make(
                "weapon",
                &w.spine_attachment_key,
                match w.id.as_str() {
                    "BugNet1" => "weapons/melee_net_".into(),
                    "MeleeChristmasTree" => "weapons/melee_CRISPRmas_tree".into(),
                    _ => format!("weapons/{}", w.spine_attachment_key),
                },
            )],
            "Gun" => w
                .spine_slots_attachments_regions
                .iter()
                .map(|[slot, template, region]| {
                    let mut region = region.clone();
                    let index = self.skin_index(e, w);
                    for (tag, names) in [
                        ("<skin>", &w.skin_names),
                        ("<skinalt>", &w.skin_names_alt),
                        ("<skin2>", &w.skin_names2),
                    ] {
                        if let Some(name) = names.get(index.min(names.len().saturating_sub(1))) {
                            region = region.replace(tag, name);
                        }
                    }
                    make(slot, template, region)
                })
                .collect(),
            "Grenade" => w
                .grenade_info
                .as_ref()
                .and_then(|g| g.spine_attachment.as_ref())
                .map(|key| vec![make("weapons-grenade", key, key.clone())])
                .unwrap_or_default(),
            _ => vec![],
        }
    }
    pub fn validate(&self, e: &EquipmentData) -> Result<(), String> {
        if e.weapon.is_some() && self.weapon(e).is_none() {
            return Err("Unknown weapon ID".into());
        }
        if e.pet
            .as_ref()
            .is_some_and(|id| !self.pets.iter().any(|pet| &pet.id == id))
        {
            return Err("Unknown pet ID".into());
        }
        if e.emote.is_some() && self.emote(e).is_none() {
            return Err("Unknown emote ID".into());
        }
        if e.umbrella
            .as_ref()
            .is_some_and(|id| !self.umbrellas.iter().any(|u| u.inventory_id == *id))
        {
            return Err("Unknown umbrella ID".into());
        }
        for (id, skin) in &e.gun_skins {
            if !self
                .weapons
                .iter()
                .any(|w| &w.id == id && w.skin_names.contains(skin))
            {
                return Err(format!("Unknown gun skin {id}/{skin}"));
            }
        }
        if let Some(loadout) = &e.loadout {
            if loadout.melee.as_ref().is_some_and(|id| {
                !self
                    .weapons
                    .iter()
                    .any(|w| &w.id == id && w.weapon_class == "Melee")
            }) {
                return Err("Unknown configured melee ID".into());
            }
            if !self.emotes.iter().any(|emote| emote.id == loadout.emote) {
                return Err("Unknown configured emote ID".into());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn categories_preserve_original_items_without_synthetic_none() {
        let catalog = Catalog::load().unwrap();
        let umbrellas = catalog
            .categories
            .iter()
            .find(|c| matches!(c.kind, CategoryKind::Umbrella))
            .unwrap();
        assert_eq!(
            umbrellas.entries.iter().map(|e| &e.id).collect::<Vec<_>>(),
            catalog
                .umbrellas
                .iter()
                .map(|e| &e.inventory_id)
                .collect::<Vec<_>>()
        );
        let pets = catalog
            .categories
            .iter()
            .find(|c| matches!(c.kind, CategoryKind::Pet))
            .unwrap();
        assert_eq!(
            pets.entries.iter().map(|e| &e.id).collect::<Vec<_>>(),
            catalog.pets.iter().map(|e| &e.id).collect::<Vec<_>>()
        );
        assert!(
            catalog
                .categories
                .iter()
                .flat_map(|c| &c.entries)
                .all(|e| e.id != "None")
        );
        let mut equipment = EquipmentData::default();
        catalog.initialize_loadout(&mut equipment);
        assert_eq!(equipment.umbrella.as_deref(), Some("UmbrellaBaseRed"));
        assert_eq!(equipment.pet.as_deref(), Some("Pet_**Null**"));
        for category in 0..catalog.categories.len() {
            assert_eq!(catalog.selected(&equipment, category), 0);
        }
    }
    #[test]
    fn flat_weapons_use_stable_skins_and_composed_icon_paths() {
        let catalog = Catalog::load().unwrap();
        assert_eq!(
            catalog
                .categories
                .iter()
                .filter(|c| c.label.starts_with("Weapon-"))
                .count(),
            27
        );
        assert_eq!(catalog.categories[0].label, "Weapon-Melee");
        assert!(
            catalog
                .categories
                .iter()
                .flat_map(|c| &c.entries)
                .all(|entry| {
                    entry
                        .weapon
                        .is_none_or(|index| catalog.weapons[index].weapon_class != "Grenade")
                })
        );
        assert_eq!(catalog.categories[0].entries.len(), 386);
        let ak = catalog
            .categories
            .iter()
            .position(|c| c.label == "Weapon-AK47")
            .unwrap();
        let mut a = EquipmentData::default();
        let b = EquipmentData::default();
        catalog.choose(&mut a, ak, 2);
        assert!(a.weapon.is_none());
        assert!(b.weapon.is_none());
        assert_eq!(catalog.selected(&a, ak), 2);
        for c in &catalog.categories {
            for e in &c.entries {
                assert!(
                    std::path::Path::new(&crate::gui::icons::path(&e.icon)).exists(),
                    "missing {}",
                    e.icon
                );
            }
        }
        a.weapon = Some("GunAK".into());
        assert!(
            catalog
                .weapon_parts(&a)
                .iter()
                .all(|p| !p.region.contains('<'))
        );
        catalog.validate(&a).unwrap();
    }

    #[test]
    fn preview_loadout_keeps_each_gun_skin_and_never_selects_booth_equipment() {
        let catalog = Catalog::load().unwrap();
        let mut equipment = EquipmentData {
            weapon: Some("GunPistol".into()),
            emote: Some("EmoteWave".into()),
            ..Default::default()
        };
        for (category, c) in catalog.categories.iter().enumerate() {
            if matches!(c.kind, CategoryKind::Weapon) {
                let chosen = c.entries.len() - 1;
                catalog.choose(&mut equipment, category, chosen);
                assert_eq!(catalog.selected(&equipment, category), chosen);
            } else if matches!(c.kind, CategoryKind::Emote) {
                assert_eq!(c.entries.len(), 74);
                assert_eq!(c.entries[0].id, "EmoteDanceBasic");
                for (index, e) in c.entries.iter().enumerate() {
                    assert_eq!(e.id, catalog.emotes[index].id);
                }
                catalog.choose(&mut equipment, category, 0);
                assert_eq!(catalog.configured_emote(&equipment), "EmoteDanceBasic");
                assert_eq!(catalog.selected(&equipment, category), 0);
            }
        }
        assert_eq!(equipment.weapon.as_deref(), Some("GunPistol"));
        assert_eq!(equipment.emote.as_deref(), Some("EmoteWave"));
        for (category, c) in catalog
            .categories
            .iter()
            .enumerate()
            .filter(|(_, c)| c.label.starts_with("Weapon-") && c.label != "Weapon-Melee")
        {
            assert_eq!(catalog.selected(&equipment, category), c.entries.len() - 1);
        }
        let booth = catalog.booth_weapons(&equipment);
        assert_eq!(booth.len(), 27);
        assert!(booth.iter().all(|entry| {
            entry
                .weapon
                .is_none_or(|i| catalog.weapons[i].weapon_class != "Grenade")
        }));
        catalog.validate(&equipment).unwrap();
        let mut other = equipment.clone();
        catalog.choose(&mut other, 0, 0);
        assert_eq!(catalog.configured_melee(&other), Some("MeleeKatana"));
        assert!(catalog.configured_melee(&equipment).is_some());
    }

    #[test]
    fn legacy_selection_migrates_without_changing_actual_equipment() {
        let catalog = Catalog::load().unwrap();
        let mut equipment: EquipmentData = serde_json::from_str(
            r#"{
            "weapon":"MeleeKatana", "emote":"EmoteWave", "gun_skins":{"GunAK":"grey"}
        }"#,
        )
        .unwrap();
        assert!(equipment.loadout.is_none());
        catalog.initialize_loadout(&mut equipment);
        assert_eq!(equipment.weapon.as_deref(), Some("MeleeKatana"));
        assert_eq!(equipment.emote.as_deref(), Some("EmoteWave"));
        assert_eq!(catalog.configured_melee(&equipment), Some("MeleeKatana"));
        assert_eq!(catalog.configured_emote(&equipment), "EmoteWave");
        catalog.validate(&equipment).unwrap();
        equipment.loadout.as_mut().unwrap().emote = "missing".into();
        assert!(catalog.validate(&equipment).is_err());
        let mut empty = EquipmentData::default();
        catalog.initialize_loadout(&mut empty);
        assert_eq!(catalog.configured_emote(&empty), "EmoteDanceBasic");
        assert!(catalog.emote(&empty).is_none());
        assert!(empty.weapon.is_none());
    }
}
