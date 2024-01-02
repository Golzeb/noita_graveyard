use std::{collections::HashMap, fmt::Display, path::PathBuf};

use xmltree::Element;

#[derive(Debug)]
pub struct BoneWand {
    pub filename: String,
    pub wand: Wand,
}

#[derive(Debug)]
pub struct Wand {
    pub name: String,
    pub shuffle: bool,
    pub spells_per_cast: i32,
    pub cast_delay: f32,
    pub recharge_time: f32,
    pub mana_max: i32,
    pub mana_charge_speed: i32,
    pub capacity: i32,
    pub spread: f32,
    pub speed: f32,
    pub tier: i32,
    pub spells: Vec<Spell>,
}

#[derive(Debug, Clone)]
pub struct Spell {
    pub name: String,
    pub always_cast: bool,
}

impl Display for Spell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name.as_str())
    }
}

impl BoneWand {
    pub fn load_from(path: &PathBuf, translation: &HashMap<String, String>) -> BoneWand {
        let element = Element::parse(std::fs::read_to_string(path).unwrap().as_bytes()).unwrap();

        let wand = Wand::load_from(element, translation);

        Self {
            filename: path.file_name().unwrap().to_str().unwrap().to_owned(),
            wand,
        }
    }
}

impl Wand {
    pub fn load_from(element: Element, translation: &HashMap<String, String>) -> Self {
        let ability_component = element.get_child("AbilityComponent").unwrap();

        let gun_config = ability_component.get_child("gun_config").unwrap();
        let gunaction_config = ability_component.get_child("gunaction_config").unwrap();

        let spells: Vec<Spell> = element
            .children
            .iter()
            .filter(|child| {
                child
                    .as_element()
                    .unwrap()
                    .attributes
                    .get("tags")
                    .unwrap_or(&"".to_owned())
                    .contains("card_action")
            })
            .map(|child| {
                let element = child.as_element().unwrap();

                let action_id = element
                    .get_child("ItemActionComponent")
                    .unwrap()
                    .attributes
                    .get("action_id")
                    .unwrap()
                    .to_lowercase();

                let name = translation
                    .get(&format!("action_{}", action_id))
                    .unwrap_or(&"???".to_owned())
                    .to_owned();

                let always_cast = parse_bool(
                    element
                        .get_child("ItemComponent")
                        .unwrap()
                        .attributes
                        .get("permanently_attached")
                        .unwrap(),
                );

                Spell { name, always_cast }
            })
            .collect();

        let name = ability_component.attributes.get("ui_name").unwrap().clone();
        let shuffle = parse_bool(
            gun_config
                .attributes
                .get("shuffle_deck_when_empty")
                .unwrap(),
        );
        let spells_per_cast = parse_i32(gun_config.attributes.get("actions_per_round").unwrap());
        let cast_delay =
            parse_f32(gunaction_config.attributes.get("fire_rate_wait").unwrap()) / 60.0;
        let recharge_time = parse_f32(gun_config.attributes.get("reload_time").unwrap()) / 60.0;
        let mana_max = parse_f32(ability_component.attributes.get("mana_max").unwrap()) as i32;
        let mana_charge_speed = parse_f32(
            ability_component
                .attributes
                .get("mana_charge_speed")
                .unwrap(),
        ) as i32;
        let capacity = parse_i32(gun_config.attributes.get("deck_capacity").unwrap());
        let spread = parse_f32(gunaction_config.attributes.get("spread_degrees").unwrap());
        let speed = parse_f32(gunaction_config.attributes.get("speed_multiplier").unwrap());
        let tier = parse_i32(
            ability_component
                .attributes
                .get("gun_level")
                .unwrap_or(&"0".to_owned()),
        );

        Self {
            name,
            shuffle,
            spells_per_cast,
            cast_delay,
            recharge_time,
            mana_max,
            mana_charge_speed,
            capacity,
            spread,
            speed,
            tier,
            spells,
        }
    }
}

fn parse_bool(string: &str) -> bool {
    let int = parse_i32(string);
    int == 1
}

fn parse_i32(string: &str) -> i32 {
    i32::from_str_radix(string, 10).unwrap()
}

fn parse_f32(string: &str) -> f32 {
    string.parse::<f32>().unwrap()
}

