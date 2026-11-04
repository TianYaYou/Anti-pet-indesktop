use anti_pet_desktop::core::store::{SettingsAction, SettingsStore, StoreEvent};
use anti_pet_desktop::state_watcher::PetSettings;
use std::path::PathBuf;

#[test]
fn test_settings_store_parity_scale_and_theme() {
    let mut store = SettingsStore::new(PathBuf::from("."), PetSettings::default());

    // Scale stepping and clamping parity
    store.dispatch(SettingsAction::SetScale(1.2345));
    assert_eq!(store.snapshot().temp_settings.scale, 1.25);

    store.dispatch(SettingsAction::SetScale(2.5));
    assert_eq!(store.snapshot().temp_settings.scale, 2.0);

    store.dispatch(SettingsAction::SetScale(0.1));
    assert_eq!(store.snapshot().temp_settings.scale, 0.5);

    // Theme cycling parity: system -> dark -> light -> system
    assert_eq!(store.snapshot().temp_settings.theme, "system");
    store.dispatch(SettingsAction::CycleTheme);
    assert_eq!(store.snapshot().temp_settings.theme, "dark");
    store.dispatch(SettingsAction::CycleTheme);
    assert_eq!(store.snapshot().temp_settings.theme, "light");
    store.dispatch(SettingsAction::CycleTheme);
    assert_eq!(store.snapshot().temp_settings.theme, "system");
}

#[test]
fn test_settings_store_parity_menu_config_toggles() {
    let mut store = SettingsStore::new(PathBuf::from("."), PetSettings::default());

    // Verify all 9 menu toggles flip correctly
    for i in 0..=8 {
        store.dispatch(SettingsAction::ToggleMenuOption(i));
    }

    let snap = store.snapshot();
    assert!(!snap.temp_settings.menu_config.open_antigravity);
    assert!(!snap.temp_settings.menu_config.settings);
    assert!(!snap.temp_settings.menu_config.random_dialogue);
    assert!(!snap.temp_settings.menu_config.pet_action);
    assert!(!snap.temp_settings.menu_config.quota);
    assert!(!snap.temp_settings.menu_config.dnd);
    assert!(!snap.temp_settings.menu_config.sound);
    assert!(!snap.temp_settings.menu_config.zoom);
    assert!(!snap.temp_settings.menu_config.gravity);

    // Toggle back
    for i in 0..=8 {
        store.dispatch(SettingsAction::ToggleMenuOption(i));
    }
    let snap2 = store.snapshot();
    assert!(snap2.temp_settings.menu_config.open_antigravity);
    assert!(snap2.temp_settings.menu_config.settings);
    assert!(snap2.temp_settings.menu_config.sound);
}

#[test]
fn test_settings_store_parity_dialogue_crud_and_rollback() {
    let mut store = SettingsStore::new(PathBuf::from("."), PetSettings::default());

    // Switch category to Petting (1)
    store.dispatch(SettingsAction::SelectDialogueCategory(1));
    assert_eq!(store.snapshot().dialogue_cat, 1);
    let original_len = store.active_dialogues().len();

    // Add empty string should be rejected by contract
    store.dispatch(SettingsAction::AddDialogue("   ".to_string()));
    assert_eq!(store.active_dialogues().len(), original_len);

    // Add valid dialogue
    store.dispatch(SettingsAction::AddDialogue("双端契约测试台词".to_string()));
    assert_eq!(store.active_dialogues().len(), original_len + 1);
    assert!(store.is_dirty());

    // Update dialogue
    store.dispatch(SettingsAction::UpdateDialogue(original_len, "修改后的契约台词".to_string()));
    assert_eq!(store.active_dialogues()[original_len], "修改后的契约台词");

    // Cancel / Revert should discard all changes and restore original state
    let ev = store.dispatch(SettingsAction::Cancel);
    assert_eq!(ev, StoreEvent::CloseWindow);
    assert!(!store.is_dirty());
    assert_eq!(store.active_dialogues().len(), original_len);
}

#[test]
fn test_settings_store_parity_save_commit() {
    let temp_dir = std::env::temp_dir().join(format!("pet_contract_test_{}", fastrand::u64(..)));
    let _ = std::fs::create_dir_all(&temp_dir);

    let mut store = SettingsStore::new(temp_dir.clone(), PetSettings::default());

    store.dispatch(SettingsAction::SetScale(1.5));
    store.dispatch(SettingsAction::ToggleSound);
    assert!(store.is_dirty());

    let ev = store.dispatch(SettingsAction::Save);
    if let StoreEvent::SettingsSaved(committed) = ev {
        assert_eq!(committed.scale, 1.5);
        assert!(!committed.sound_enabled);
        assert!(!store.is_dirty());
        assert_eq!(store.snapshot().original_settings.scale, 1.5);
    } else {
        panic!("Expected StoreEvent::SettingsSaved");
    }

    let _ = std::fs::remove_dir_all(&temp_dir);
}
