use gtps_world::World;

use super::{economy, inventory::Inventory, tools};

pub fn exchange_only(item: u16) -> bool {
    item == economy::SPRAY_TOKEN_ICON || item == economy::SPRAY_TOKEN_ICON + 1
}

pub fn allows_world_reward(item: u16) -> bool {
    !tools::shop_only(item) && !exchange_only(item)
}

pub fn collect(world: &mut World, inventory: &mut Inventory, uid: u32, x: f32, y: f32) -> bool {
    let Some(object) = world
        .objects
        .iter()
        .find(|object| object.uid == uid)
        .copied()
    else {
        return false;
    };
    if !allows_world_reward(object.item_id)
        || !x.is_finite()
        || !y.is_finite()
        || !object.pos_x.is_finite()
        || !object.pos_y.is_finite()
        || (object.pos_x - x).abs() > 64.0
        || (object.pos_y - y).abs() > 64.0
    {
        return false;
    }
    let Ok(count) = u8::try_from(object.count) else {
        return false;
    };
    if !inventory.add(object.item_id, count) {
        return false;
    }
    world.remove_object(uid);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collection_preserves_objects_when_full_and_rejects_replay() {
        let mut world = World::empty("TEST");
        let mut inventory = Inventory::default();
        inventory.add(2, 250);
        let uid = world.add_object(2, 16.0, 16.0, 1);
        assert!(!collect(&mut world, &mut inventory, uid, 16.0, 16.0));
        assert_eq!(world.objects.len(), 1);
        inventory.remove(2, 1);
        assert!(!collect(&mut world, &mut inventory, uid, 1000.0, 16.0));
        assert!(collect(&mut world, &mut inventory, uid, 16.0, 16.0));
        assert!(!collect(&mut world, &mut inventory, uid, 16.0, 16.0));
        assert_eq!(inventory.count_of(2), 250);
    }

    #[test]
    fn equipment_and_tourney_tokens_cannot_be_collected() {
        let mut world = World::empty("TEST");
        let mut inventory = Inventory::default();
        for item in tools::CATALOG
            .iter()
            .map(|tool| tool.item)
            .chain([economy::SPRAY_TOKEN_ICON])
        {
            for id in [item, item + 1] {
                let uid = world.add_object(id, 0.0, 0.0, 1);
                assert!(!collect(&mut world, &mut inventory, uid, 0.0, 0.0));
                assert_eq!(inventory.count_of(id), 0);
            }
        }
        assert!(allows_world_reward(1486));
        assert!(allows_world_reward(7188));
    }
}
