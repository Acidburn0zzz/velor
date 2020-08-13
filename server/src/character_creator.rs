use specs::{Entity, ReadExpect};
use crate::persistence;
use crate::persistence::character::CharacterLoader;
use common::loadout_builder::LoadoutBuilder;
use common::comp::{Body, Inventory, Stats};

pub fn create_character(entity: Entity,
                        player_uuid: String,
                        character_alias: String,
                        character_tool: Option<String>,
                        body: Body,
                        character_loader: &ReadExpect<'_, CharacterLoader>) {
    let stats = Stats::new(character_alias.to_string(), body);

    let loadout = LoadoutBuilder::new()
        .defaults()
        .active_item(LoadoutBuilder::default_item_config_from_str(character_tool.as_deref().unwrap()))
        .build();

    let inventory = Inventory::default();

    character_loader.create_character(entity, player_uuid, character_alias, stats, loadout, inventory, body);
}