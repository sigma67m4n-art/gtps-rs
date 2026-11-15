use gtps_proto::{Text, Variant, VariantList};

use crate::game::cheats::CheatState;
use crate::game::economy;
use crate::game::locks::{self, LockKind, LockSpec, TELEPHONE_ITEM};

pub const DIALOG_TELEPHONE_DIAL: &str = "telephone_dial";
pub const DIALOG_TELEPHONE: &str = "telephone";
pub const DIALOG_CHEATS: &str = "cheats";

pub const SERVICE_NUMBER: &str = "53785";
pub const BUY_DIAMOND: &str = "buy_diamond";
pub const BUY_BLUE_GEM: &str = "buy_bluegem";
pub const CONVERT_BLUE_GEM: &str = "convert_bluegem";
pub const EXCHANGE_TOKENS: &str = "exchange_tokens";

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
pub fn dial_dialog() -> VariantList {
    let mut body = String::new();
    body.push_str(&format!(
        "add_label_with_icon|big|`wTelephone``|left|{TELEPHONE_ITEM}|\n"
    ));
    body.push_str("add_textbox|`wDial a number to reach a service.``|left|\n");
    body.push_str("add_text_input|number|Number||12|\n");
    body.push_str("add_button|call|`wCall``|\n");
    body.push_str("add_button|close|`wClose``|\n");
    body.push_str(&format!("end_dialog|{DIALOG_TELEPHONE_DIAL}|Call|\n"));

    VariantList::new("OnDialogRequest").arg(Variant::String(body))
}

#[must_use]
pub fn telephone_dialog(gems: u32, credits: u64, growtokens: u32) -> VariantList {
    let diamond = locks::by_kind(LockKind::Diamond);
    let blue_gem = locks::by_kind(LockKind::BlueGem);
    let diamond_total = locks::total_with_tax(diamond.shop_price);
    let blue_gem_total = locks::total_with_tax(blue_gem.shop_price);

    let mut body = String::new();
    body.push_str(&format!(
        "add_label_with_icon|big|`wLock Service``|left|{TELEPHONE_ITEM}|\n"
    ));
    body.push_str(&format!(
        "add_textbox|`wGems: `9{gems}`` `wTax Credit: `9{credits}`` `wGrowtokens: `9{growtokens}``|left|\n"
    ));
    body.push_str("add_spacer|small|\n");
    body.push_str(&format!(
        "add_button|{BUY_DIAMOND}|`w{} ``(`9{} Gems``)``|\n",
        diamond.name,
        with_commas(diamond_total)
    ));
    body.push_str(&format!(
        "add_button|{BUY_BLUE_GEM}|`w{} ``(`9{} Gems``)``|\n",
        blue_gem.name,
        with_commas(blue_gem_total)
    ));
    body.push_str(&format!(
        "add_button|{CONVERT_BLUE_GEM}|`wConvert ``(`9{} Diamond Locks``)`w into a {}\n",
        locks::DIAMOND_PER_BLUE_GEM,
        blue_gem.name
    ));
    body.push_str(&format!(
        "add_button|{EXCHANGE_TOKENS}|`wConvert Tax to Growtokens ``(`9{} per token``)``|\n",
        with_commas(economy::GEMS_PER_GROWTOKEN)
    ));
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
pub fn is_service_number(number: &str) -> bool {
    number.trim() == SERVICE_NUMBER
}

#[must_use]
pub fn lock_button(text: &Text) -> Option<&'static LockSpec> {
    if text.has(BUY_DIAMOND) {
        return Some(locks::by_kind(LockKind::Diamond));
    }

    if text.has(BUY_BLUE_GEM) {
        return Some(locks::by_kind(LockKind::BlueGem));
    }

    None
}

#[must_use]
pub fn wants_conversion(text: &Text) -> bool {
    text.has(CONVERT_BLUE_GEM)
}

#[must_use]
pub fn wants_exchange(text: &Text) -> bool {
    text.has(EXCHANGE_TOKENS)
}

#[must_use]
pub fn cheat_button(text: &Text) -> Option<&'static str> {
    text.iter()
        .find_map(|(key, _)| CheatState::from_button(key))
}

#[cfg(test)]
mod tests {
    use super::{
        cheat_button, cheats_dialog, is_service_number, lock_button, telephone_dialog,
        wants_conversion, wants_exchange, with_commas, BUY_BLUE_GEM, CONVERT_BLUE_GEM,
        EXCHANGE_TOKENS,
    };
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
    fn only_the_service_number_is_valid() {
        assert!(is_service_number("53785"));
        assert!(is_service_number(" 53785 "));
        assert!(!is_service_number("00000"));
    }

    #[test]
    fn telephone_offers_buy_convert_and_exchange() {
        let text = body(&telephone_dialog(500, 7_500, 3));
        assert!(text.contains(BUY_BLUE_GEM));
        assert!(text.contains(CONVERT_BLUE_GEM));
        assert!(text.contains(EXCHANGE_TOKENS));
        assert!(text.contains("105,000 Gems"));
        assert!(text.contains("10,500,000 Gems"));
        assert!(text.contains("end_dialog|telephone|"));
    }

    #[test]
    fn cheats_dialog_lists_options() {
        let text = body(&cheats_dialog(&CheatState::default()));
        assert!(text.contains("cheat_auto_farm"));
        assert!(text.contains("cheat_bfg_mode"));
        assert!(text.contains("end_dialog|cheats|"));
    }

    #[test]
    fn dialog_return_maps_buttons() {
        let buy = Text::parse_str("dialog_name|telephone\nbuy_bluegem|\n");
        assert_eq!(
            lock_button(&buy).map(|spec| spec.kind),
            Some(LockKind::BlueGem)
        );
        assert!(!wants_conversion(&buy));
        assert!(!wants_exchange(&buy));

        let convert = Text::parse_str("dialog_name|telephone\nconvert_bluegem|\n");
        assert!(wants_conversion(&convert));
        assert!(lock_button(&convert).is_none());

        let exchange = Text::parse_str("dialog_name|telephone\nexchange_tokens|\n");
        assert!(wants_exchange(&exchange));

        let cheats = Text::parse_str("dialog_name|cheats\ncheat_auto_farm|\n");
        assert_eq!(cheat_button(&cheats), Some("auto_farm"));
    }
}
