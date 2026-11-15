use gtps_proto::{Text, Variant, VariantList};

use crate::game::cheats::CheatState;
use crate::game::locks::{self, LockKind, LockSpec, TELEPHONE_ITEM};

pub const DIALOG_TELEPHONE: &str = "telephone";
pub const DIALOG_CHEATS: &str = "cheats";

fn with_commas(value: u64) -> String {
    let digits = value.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);

    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            out.push(',');
        }
        out.push(digit);
    }

    out
}

#[must_use]
pub fn telephone_dialog(gems: u32) -> VariantList {
    let mut body = String::new();
    body.push_str(&format!(
        "add_label_with_icon|big|`wTelephone``|left|{TELEPHONE_ITEM}|\n"
    ));
    body.push_str(&format!(
        "add_label_with_icon|small|`wLock Shop``|left|{TELEPHONE_ITEM}|\n"
    ));
    body.push_str("add_spacer|small|\n");
    body.push_str(&format!("add_textbox|`wYour Gems: `9{gems}``|left|\n"));
    body.push_str("add_spacer|small|\n");

    for kind in [LockKind::Diamond, LockKind::BlueGem] {
        let spec = locks::by_kind(kind);
        let total = locks::total_with_tax(spec.shop_price);
        body.push_str(&format!(
            "add_button|{}|`w{} ``(`9{} Gems``)``|\n",
            spec.kind.sale_button(),
            spec.name,
            with_commas(total)
        ));
    }

    body.push_str("add_spacer|small|\n");
    body.push_str("add_button|close|`wClose``|\n");
    body.push_str(&format!("end_dialog|{DIALOG_TELEPHONE}|Close|\n"));

    VariantList::new("OnDialogRequest").arg(Variant::String(body))
}

#[must_use]
pub fn cheats_dialog(cheats: &CheatState) -> VariantList {
    let mut body = String::new();
    body.push_str("add_label_with_icon|big|`wCheats``|left|1|\n");
    body.push_str("add_textbox|`wTap an option to toggle it.``|left|\n");
    body.push_str("add_spacer|small|\n");

    for (name, label, enabled) in cheats.entries() {
        let status = if enabled { "ON" } else { "OFF" };
        body.push_str(&format!(
            "add_button|{}{name}|`w{label}: {status}``|\n",
            CheatState::BUTTON_PREFIX
        ));
    }

    body.push_str("add_spacer|small|\n");
    body.push_str("add_button|close|`wClose``|\n");
    body.push_str(&format!("end_dialog|{DIALOG_CHEATS}|Close|\n"));

    VariantList::new("OnDialogRequest").arg(Variant::String(body))
}

#[must_use]
pub fn lock_button(text: &Text) -> Option<&'static LockSpec> {
    locks::for_sale().find(|spec| text.has(spec.kind.sale_button()))
}

#[must_use]
pub fn cheat_button(text: &Text) -> Option<&'static str> {
    text.iter()
        .find_map(|(key, _)| CheatState::from_button(key))
}

#[cfg(test)]
mod tests {
    use super::{cheat_button, cheats_dialog, lock_button, telephone_dialog, with_commas};
    use crate::game::cheats::CheatState;
    use crate::game::locks::LockKind;
    use gtps_proto::{Text, Variant};

    fn body(list: &gtps_proto::VariantList) -> String {
        match &list.args()[0] {
            Variant::String(text) => text.clone(),
            other => panic!("unexpected variant: {other:?}"),
        }
    }

    #[test]
    fn commas_group_digits() {
        assert_eq!(with_commas(100_000), "100,000");
        assert_eq!(with_commas(10_500_000), "10,500,000");
        assert_eq!(with_commas(999), "999");
    }

    #[test]
    fn telephone_offers_taxed_locks() {
        let text = body(&telephone_dialog(500));
        assert!(text.contains("buy_diamond"));
        assert!(text.contains("buy_bluegem"));
        assert!(text.contains("100,000 Gems"));
        assert!(text.contains("10,500,000 Gems"));
        assert!(text.contains("end_dialog|telephone|"));
    }

    #[test]
    fn cheats_dialog_lists_options() {
        let text = body(&cheats_dialog(&CheatState::default()));
        assert!(text.contains("cheat_auto_farm"));
        assert!(text.contains("cheat_bfg_mode"));
        assert!(text.contains("cheat_auto_grab_remote"));
        assert!(text.contains("end_dialog|cheats|"));
    }

    #[test]
    fn dialog_return_maps_buttons() {
        let telephone = Text::parse_str("dialog_name|telephone\nbuy_bluegem|\n");
        assert_eq!(
            lock_button(&telephone).map(|spec| spec.kind),
            Some(LockKind::BlueGem)
        );

        let cheats = Text::parse_str("dialog_name|cheats\ncheat_auto_farm|\n");
        assert_eq!(cheat_button(&cheats), Some("auto_farm"));
    }
}
