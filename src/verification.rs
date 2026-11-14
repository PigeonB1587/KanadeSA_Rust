//! Opt-in desktop rendering smoke check (`--verify-booth OUTPUT_DIRECTORY`).
//! Uses the same controllers, save/load and capture paths as the interactive app.
use crate::{
    booth::Booth,
    gui,
    model::{Action, ReferenceFrame, Watermark},
};
use macroquad::prelude::*;
use std::path::Path;

pub async fn run(
    booth: &mut Booth,
    directory: &Path,
    font: &Font,
    bold: &Font,
) -> Result<(), String> {
    std::fs::create_dir_all(directory).map_err(|e| e.to_string())?;
    booth.save_directory = directory.join("save");
    // Separate animals/equipment, mirrored pets, umbrella remaps, GEM animation.
    let catalog = booth.characters[0].wardrobe().equipment_catalog.clone();
    let ak = catalog
        .categories
        .iter()
        .position(|c| c.label == "Weapon-AK47")
        .ok_or("AK47 category missing")?;
    booth.characters[0].choose_equipment(ak, 2)?;
    booth.characters[0].use_weapon(Some("GunAK"))?;
    let pet = catalog
        .categories
        .iter()
        .position(|c| matches!(c.kind, crate::equipment::CategoryKind::Pet))
        .unwrap();
    let umbrella = catalog
        .categories
        .iter()
        .position(|c| matches!(c.kind, crate::equipment::CategoryKind::Umbrella))
        .unwrap();
    let emote = catalog
        .categories
        .iter()
        .position(|c| matches!(c.kind, crate::equipment::CategoryKind::Emote))
        .unwrap();
    booth.characters[0].choose_equipment(pet, 1)?;
    booth.characters[0].placement.position = [-3.0, -3.0];
    booth.characters[0].placement.height = -3.0;
    booth.characters[0].placement.color = [0.8, 1.0, 0.7, 1.0];
    booth.add()?;
    booth.characters[1].choose(None, 100);
    booth.characters[1].choose_equipment(umbrella, 12)?;
    booth.characters[1].set_action(Action::Umbrella);
    booth.characters[1].placement.position = [0.0, -1.5];
    booth.characters[1].placement.height = -1.5;
    booth.characters[1].placement.face_right = false;
    booth.characters[1].placement.pet_face_right = false;
    booth.add()?;
    booth.characters[2].choose(None, 300);
    let gem = catalog.categories[emote]
        .entries
        .iter()
        .position(|e| e.id.to_lowercase().contains("gem"))
        .ok_or("Gem emote missing")?;
    booth.characters[2].choose_equipment(emote, gem)?;
    booth.characters[2].use_emote(&catalog.categories[emote].entries[gem].id)?;
    booth.characters[2].level = 999;
    booth.characters[2].placement.position = [3.0, -3.0];
    booth.characters[2].placement.height = -3.0;
    booth.characters[2].placement.pet_reference = ReferenceFrame::World;
    booth.characters[2].placement.pet_position = [4.6, -3.5];
    for _ in 0..45 {
        booth.update(1.0 / 60.0);
    }
    for c in &mut booth.characters {
        c.animation.paused = true;
    }
    // Editing a separate Preview must change configuration, never the booth's
    // held weapon/emote, action, transforms or unrelated track progress.
    let before_preview = booth.characters[0].data();
    let placement_before_preview = booth.characters[0].placement.clone();
    // Preview must ignore every scene-only property, not just mirror state.
    let original_placement = booth.characters[0].placement.clone();
    booth.characters[0].placement.face_right = false;
    booth.characters[0].placement.pet_face_right = false;
    booth.characters[0].placement.rotation = 37.0;
    booth.characters[0].placement.target_angle = 62.0;
    booth.characters[0].placement.scale = 2.4;
    booth.characters[0].placement.color = [0.2, 0.4, 0.7, 0.3];
    booth.characters[0].placement.pet_reference = ReferenceFrame::World;
    booth.characters[0].placement.pet_position = [8.0, 9.0];
    let mut preview = booth.characters[0].preview_copy()?;
    let expected_placement = crate::model::Placement {
        character_id: preview.id.clone(),
        ..Default::default()
    };
    if preview.placement != expected_placement
        || preview.animation.action != Action::Idle
        || preview.animation.paused
        || preview
            .animation
            .tracks
            .iter()
            .any(|track| track.time != 0.0 || track.speed != 1.0)
    {
        return Err("Preview inherited scene transforms, tint, pet placement or playback".into());
    }
    booth.characters[0].placement = original_placement;
    let pistol = catalog
        .categories
        .iter()
        .position(|c| c.label == "Weapon-Pistol")
        .ok_or("Pistol category missing")?;
    preview.choose_equipment(pistol, 1)?;
    preview.preview_category(pistol);
    preview.choose_equipment(emote, gem)?;
    preview.preview_category(emote);
    if preview.data().equipment.weapon.as_deref() != Some("GunPistol")
        || preview.animation.action != Action::Emote
    {
        return Err("Preview did not try configured equipment".into());
    }
    booth.characters[0].apply_preview(&preview)?;
    let applied = booth.characters[0].data();
    if applied.equipment.weapon != before_preview.equipment.weapon
        || applied.equipment.emote != before_preview.equipment.emote
        || applied.animation != before_preview.animation
        || booth.characters[0].placement != placement_before_preview
        || applied.equipment.loadout.as_ref().map(|l| l.emote.as_str())
            != Some(catalog.categories[emote].entries[gem].id.as_str())
    {
        return Err("Preview changed booth selection or progress".into());
    }
    // Reopening Preview, duplication and saves retain all per-gun choices.
    if booth.characters[0].duplicate()?.data().equipment != applied.equipment {
        return Err("Duplicated character lost configured or actual equipment".into());
    }
    // Explicit booth selection changes only the relevant runtime tracks.
    let run_time = booth.characters[0].animation.tracks[0].clone();
    booth.characters[0].use_weapon(Some("GunPistol"))?;
    if booth.characters[0].animation.tracks[0] != run_time {
        return Err("Booth weapon selection reset unrelated track".into());
    }
    booth.characters[0].use_weapon(Some("GunAK"))?;
    booth.capture_to(&directory.join("restore-before.png"), 1280, 720)?;
    booth.save()?;
    let before: Vec<_> = booth.characters.iter().map(|c| c.data()).collect();
    let placements: Vec<_> = booth
        .characters
        .iter()
        .map(|c| c.placement.clone())
        .collect();
    booth.load()?;
    if booth
        .characters
        .iter()
        .map(|c| c.data())
        .collect::<Vec<_>>()
        != before
    {
        return Err("Runtime character restore mismatch".into());
    }
    if booth
        .characters
        .iter()
        .map(|c| c.placement.clone())
        .collect::<Vec<_>>()
        != placements
    {
        return Err("Runtime placement restore mismatch".into());
    }
    booth.capture_to(&directory.join("restore-after.png"), 1280, 720)?;
    let before_image = image::open(directory.join("restore-before.png"))
        .map_err(|e| e.to_string())?
        .into_rgba8();
    let after_image = image::open(directory.join("restore-after.png"))
        .map_err(|e| e.to_string())?
        .into_rgba8();
    if before_image != after_image {
        return Err("Paused save/load changed rendered pixels".into());
    }
    // A semantically invalid save must leave every live entity untouched.
    let (mut invalid, scene) = crate::persistence::load(&booth.save_directory)?;
    invalid.characters[0].animation.tracks[0].token = "missing/animation".into();
    let invalid_directory = directory.join("invalid-save");
    crate::persistence::save(&invalid_directory, &invalid, &scene)?;
    booth.save_directory = invalid_directory;
    if booth.load().is_ok()
        || booth
            .characters
            .iter()
            .map(|c| c.data())
            .collect::<Vec<_>>()
            != before
    {
        return Err("Failed load did not preserve live scene".into());
    }
    booth.save_directory = directory.join("save");
    for index in 0..booth.backgrounds.len() {
        booth.switch_background(index);
        booth.capture_to(&directory.join(format!("scene-{index}.png")), 1280, 720)?;
    }
    booth.switch_background(0);
    booth.transparent = true;
    booth.watermark = Watermark::Left;
    booth.capture_to(&directory.join("transparent.png"), 1280, 720)?;
    booth.switch_background(1);
    booth.watermark = Watermark::Right;
    booth.render(1280, 720, false)?;
    clear_background(Color::from_rgba(24, 24, 32, 255));
    gui::layout::begin();
    let mut ui = gui::booth::BoothUi::default();
    ui.draw_canvas(booth);
    ui.draw(booth, font, bold, false);
    let mut settings = gui::settings::Settings::default();
    settings.booth_mode = true;
    settings.draw(booth.selected_mut().unwrap(), font, bold, false);
    screen(directory, "booth-ui")?;
    next_frame().await;
    let mut selection = gui::selection::Selection::default();
    for (width, height) in [
        (1280.0, 720.0),
        (640.0, 360.0),
        (480.0, 720.0),
        (480.0, 320.0),
    ] {
        request_new_screen_size(width, height);
        for _ in 0..4 {
            next_frame().await;
        }
        clear_background(Color::from_rgba(24, 24, 32, 255));
        gui::layout::begin();
        ui.draw_canvas(booth);
        ui.draw(booth, font, bold, false);
        settings.booth_mode = true;
        settings.draw(booth.selected_mut().unwrap(), font, bold, false);
        screen(directory, &format!("booth-{width}x{height}"))?;
        next_frame().await;
        let mut picker = gui::equipment::EquipmentPicker::default();
        for (kind, name) in [
            (gui::equipment::Kind::Weapon, "weapons"),
            (gui::equipment::Kind::Emote, "emotes"),
        ] {
            picker.open(kind, &booth.selected().unwrap().id);
            for frame in 0..20 {
                clear_background(Color::from_rgba(24, 24, 32, 255));
                gui::layout::begin();
                ui.draw_canvas(booth);
                picker.toolbar(booth, ui.rects().0, font, vec2(-10000.0, -10000.0));
                picker.draw(booth, font, bold, vec2(-10000.0, -10000.0));
                if frame == 19 {
                    screen(directory, &format!("picker-{name}-{width}x{height}"))?;
                }
                next_frame().await;
            }
        }
        for (index, name) in [(1, "umbrella"), (2, "gem")] {
            booth.selected = Some(index);
            clear_background(Color::from_rgba(24, 24, 32, 255));
            booth.draw_preview()?;
            let c = booth.selected_mut().unwrap();
            gui::layout::begin();
            gui::preview::draw(c, font, bold);
            selection.draw(c, font, bold).await;
            ui.preview_close(font, false);
            settings.booth_mode = false;
            settings.draw(c, font, bold, false);
            let mut info = gui::preview::Info::default();
            info.draw(c, font, bold, false);
            screen(directory, &format!("preview-{name}-{width}x{height}"))?;
            next_frame().await;
        }
    }
    // Keep a hidden resource owner when empty, then recreate a fresh identity.
    while booth.active_count() > 0 {
        booth.remove();
    }
    booth.save_directory = directory.join("empty-save");
    booth.save()?;
    booth.add_or_restore()?;
    booth.load()?;
    if booth.active_count() != 0 || booth.selected.is_some() {
        return Err("Empty scene did not restore".into());
    }
    let hidden_id = booth.characters[0].id.clone();
    booth.add_or_restore()?;
    if booth.active_count() != 1 || booth.characters[0].id == hidden_id {
        return Err("Adding to empty scene did not create independent identity".into());
    }
    next_frame().await;
    Ok(())
}

fn screen(directory: &Path, name: &str) -> Result<(), String> {
    set_default_camera();
    unsafe {
        get_internal_gl().flush();
    }
    get_screen_data().export_png(
        directory
            .join(format!("{name}.png"))
            .to_str()
            .ok_or("Invalid path")?,
    );
    Ok(())
}
