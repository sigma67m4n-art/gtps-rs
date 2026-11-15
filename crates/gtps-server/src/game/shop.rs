use gtps_proto::{Text, Variant, VariantList};

use crate::game::cheats::CheatState;
use crate::game::economy;
use crate::game::tools;
use crate::game::locks::{self, LockKind, LockSpec, TELEPHONE_ITEM};

pub const DIALOG_TELEPHONE_DIAL: &str = "telephone_dial";
pub const DIALOG_TELEPHONE: &str = "telephone";
pub const DIALOG_TAX: &str = "tax";
pub const DIALOG_SPRAY: &str = "spray";
pub const DIALOG_CHEATS: &str = "cheats";

pub const SALESMAN_NUMBER: &str = "53785";
pub const TAX_NUMBER: &str = "12345";
pub const SPRAY_NUMBER: &str = "53675";

pub const BUY_DIAMOND: &str = "buy_diamond";
pub const BUY_BLUE_GEM: &str = "buy_bluegem";
pub const CONVERT_BLUE_GEM: &str = "convert_bluegem";
pub const CONVERT_WHITE_GEM: &str = "convert_whitegem";
pub const OPEN_TAX: &str = "open_tax";
pub const OPEN_SPRAY: &str = "open_spray";
pub const EXCHANGE_TOKENS: &str = "exchange_tokens";
pub const EXCHANGE_SPRAY: &str = "exchange_spray";
pub const BUY_SPRAY: &str = "buy_spray";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Service {
    Salesman,
    Tax,
    Spray,
}

#[must_use]
pub fn service_for(number: &str) -> Option<Service> {
    match number.trim() {
        SALESMAN_NUMBER => Some(Service::Salesman),
        TAX_NUMBER => Some(Service::Tax),
        SPRAY_NUMBER => Some(Service::Spray),
        _ => None,
    }
}

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
    body.push_str(&format!(
        "add_textbox|`wSales-Man: `9{SALESMAN_NUMBER}`` `wTax: `9{TAX_NUMBER}`` `wSpray Shop: `9{SPRAY_NUMBER}``|left|\n"
    ));
    body.push_str("add_text_input|number|Number||12|\n");
    body.push_str("add_button|call|`wCall``|\n");
    body.push_str("add_button|close|`wClose``|\n");
    body.push_str(&format!("end_dialog|{DIALOG_TELEPHONE_DIAL}|Call|\n"));

    VariantList::new("OnDialogRequest").arg(Variant::String(body))
}

#[must_use]
pub fn salesman_dialog(gems: u32, credits: u64, growtokens: u32, spray_tokens: u32) -> VariantList {
    let diamond = locks::by_kind(LockKind::Diamond);
    let blue_gem = locks::by_kind(LockKind::BlueGem);
    let white_gem = locks::by_kind(LockKind::WhiteGem);
    let diamond_total = locks::total_with_tax(diamond.shop_price);
    let blue_gem_total = locks::total_with_tax(blue_gem.shop_price);

    let mut body = String::new();
    body.push_str(&format!(
        "add_label_with_icon|big|`wSales-Man``|left|{TELEPHONE_ITEM}|\n"
    ));
    body.push_str(&format!(
        "add_textbox|`wGems: `9{gems}`` `wTax Credit: `9{credits}`` `wGrowtokens: `9{growtokens}`` `wTourney Tokens: `9{spray_tokens}``|left|\n"
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
        "add_button|{CONVERT_WHITE_GEM}|`wConvert ``(`9{} Blue Gem Locks``)`w into a {}\n",
        locks::BLUE_GEM_PER_WHITE_GEM,
        white_gem.name
    ));
    body.push_str("add_spacer|small|\n");
    body.push_str(&format!(
        "add_button|{OPEN_TAX}|`wTax Service ``(`9{TAX_NUMBER}``)``|\n"
    ));
    body.push_str(&format!(
        "add_button|{OPEN_SPRAY}|`wSpray Shop ``(`9{SPRAY_NUMBER}``)``|\n"
    ));
    body.push_str("add_spacer|small|\n");
    body.push_str("add_button|close|`wClose``|\n");
    body.push_str(&format!("end_dialog|{DIALOG_TELEPHONE}|Close|\n"));

    VariantList::new("OnDialogRequest").arg(Variant::String(body))
}

#[must_use]
pub fn tax_dialog(credits: u64, growtokens: u32, spray_tokens: u32) -> VariantList {
    let mut body = String::new();
    body.push_str(&format!(
        "add_label_with_icon|big|`wTax Service``|left|{}\n",
        economy::SPRAY_TOKEN_ICON
    ));
    body.push_str(&format!(
        "add_textbox|`wTax Credit: `9{credits}`` `wGrowtokens: `9{growtokens}`` `wTourney Tokens: `9{spray_tokens}``|left|\n"
    ));
    body.push_str("add_spacer|small|\n");
    body.push_str(&format!(
        "add_button|{EXCHANGE_TOKENS}|`wConvert Tax to Growtokens ``(`9{} per token``)``|\n",
        with_commas(economy::GEMS_PER_GROWTOKEN)
    ));
    body.push_str(&format!(
        "add_button|{EXCHANGE_SPRAY}|`wConvert Tax to Tourney Tokens ``(`9{} per token``)``|\n",
        with_commas(economy::GEMS_PER_SPRAY_TOKEN)
    ));
    body.push_str("add_spacer|small|\n");
    body.push_str("add_button|close|`wClose``|\n");
    body.push_str(&format!("end_dialog|{DIALOG_TAX}|Close|\n"));

    VariantList::new("OnDialogRequest").arg(Variant::String(body))
}

#[must_use]
pub fn spray_dialog(spray_tokens: u32) -> VariantList {
    let mut body = String::new();
    body.push_str(&format!(
        "add_label_with_icon|big|`wSpray Shop``|left|{}\n",
        economy::ULTRA_WORLD_SPRAY
    ));
    body.push_str(&format!(
        "add_textbox|`wTourney Tokens: `9{spray_tokens}``|left|\n"
    ));
    body.push_str("add_textbox|`wUltra World Spray instantly grows every plant in the world.``|left|\n");
    body.push_str("add_spacer|small|\n");
    body.push_str(&format!(
        "add_button|{BUY_SPRAY}|`wUltra World Spray ``(`9{} Tourney Tokens``)``|\n",
        economy::ULTRA_WORLD_SPRAY_PRICE
    ));
    body.push_str("add_spacer|small|\n");
    body.push_str("add_button|close|`wClose``|\n");
    body.push_str(&format!("end_dialog|{DIALOG_SPRAY}|Close|\n"));

    VariantList::new("OnDialogRequest").arg(Variant::String(body))
}

pub fn equipment_dialog(gems: u32) -> VariantList {
    let mut body = format!("add_label_with_icon|big|`wEquipment Shop``|left|{}|\nadd_textbox|Gems: {gems}|left|\n", tools::DEATH_SCYTHE);
    for tool in tools::CATALOG {
        body.push_str(&format!("add_button|buy_tool_{}|{} - {} Gems|\n", tool.item, tool.name, with_commas(u64::from(tool.price))));
    }
    body.push_str(&format!("end_dialog|{}|Close||\n", tools::DIALOG));
    VariantList::new("OnDialogRequest").arg(Variant::String(body))
}

pub fn clicked(text: &Text, button: &str) -> bool {
    match text.get("buttonClicked") {
        Some(clicked) => clicked == button,
        None => text.has(button),
    }
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
    if clicked(text, BUY_DIAMOND) {
        return Some(locks::by_kind(LockKind::Diamond));
    }

    if clicked(text, BUY_BLUE_GEM) {
        return Some(locks::by_kind(LockKind::BlueGem));
    }

    None
}

#[must_use]
pub fn open_button(text: &Text) -> Option<Service> {
    if clicked(text, OPEN_TAX) {
        return Some(Service::Tax);
    }

    if clicked(text, OPEN_SPRAY) {
        return Some(Service::Spray);
    }

    None
}

#[must_use]
pub fn wants_conversion(text: &Text) -> bool {
    clicked(text, CONVERT_BLUE_GEM)
}

#[must_use]
pub fn wants_white_conversion(text: &Text) -> bool {
    clicked(text, CONVERT_WHITE_GEM)
}

#[must_use]
pub fn wants_exchange(text: &Text) -> bool {
    clicked(text, EXCHANGE_TOKENS)
}

#[must_use]
pub fn wants_spray_exchange(text: &Text) -> bool {
    clicked(text, EXCHANGE_SPRAY)
}

#[must_use]
pub fn wants_buy_spray(text: &Text) -> bool {
    clicked(text, BUY_SPRAY)
}

#[must_use]
pub fn cheat_button(text: &Text) -> Option<&'static str> {
    if let Some(button) = text.get("buttonClicked") {
        return CheatState::from_button(button);
    }
    text.iter()
        .find_map(|(key, _)| CheatState::from_button(key))
}

#[cfg(test)]
mod tests {
    use super::{
        cheat_button, cheats_dialog, lock_button, open_button, salesman_dialog, service_for,
        spray_dialog, tax_dialog, wants_buy_spray, wants_conversion, wants_exchange,
        wants_spray_exchange, wants_white_conversion, with_commas, Service, BUY_BLUE_GEM,
        BUY_SPRAY, CONVERT_BLUE_GEM, CONVERT_WHITE_GEM, EXCHANGE_SPRAY, EXCHANGE_TOKENS,
        OPEN_TAX, SALESMAN_NUMBER, SPRAY_NUMBER, TAX_NUMBER,
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
    fn telephone_numbers_map_to_services() {
        assert_eq!(service_for(SALESMAN_NUMBER), Some(Service::Salesman));
        assert_eq!(service_for(TAX_NUMBER), Some(Service::Tax));
        assert_eq!(service_for(SPRAY_NUMBER), Some(Service::Spray));
        assert_eq!(service_for(" 12345 "), Some(Service::Tax));
        assert_eq!(service_for("00000"), None);
    }

    #[test]
    fn salesman_offers_locks_and_services() {
        let text = body(&salesman_dialog(500, 7_500, 3, 1));
        assert!(text.contains(BUY_BLUE_GEM));
        assert!(text.contains(CONVERT_BLUE_GEM));
        assert!(text.contains(CONVERT_WHITE_GEM));
        assert!(text.contains(OPEN_TAX));
        assert!(text.contains("105,000 Gems"));
        assert!(text.contains("10,500,000 Gems"));
        assert!(text.contains("White Gem Lock"));
        assert!(text.contains("end_dialog|telephone|"));
    }

    #[test]
    fn tax_dialog_offers_two_tokens() {
        let text = body(&tax_dialog(3_000_000, 0, 0));
        assert!(text.contains(EXCHANGE_TOKENS));
        assert!(text.contains(EXCHANGE_SPRAY));
        assert!(text.contains("1,000,000 per token"));
        assert!(text.contains("end_dialog|tax|"));
    }

    #[test]
    fn spray_dialog_sells_ultra_world_spray() {
        let text = body(&spray_dialog(2));
        assert!(text.contains(BUY_SPRAY));
        assert!(text.contains("instant"));
        assert!(text.contains("end_dialog|spray|"));
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
        assert!(open_button(&buy).is_none());

        let convert = Text::parse_str("dialog_name|telephone\nconvert_bluegem|\n");
        assert!(wants_conversion(&convert));
        assert!(lock_button(&convert).is_none());

        let white = Text::parse_str("dialog_name|telephone\nconvert_whitegem|\n");
        assert!(wants_white_conversion(&white));
        assert!(!wants_conversion(&white));

        let tax = Text::parse_str("dialog_name|telephone\nopen_tax|\n");
        assert_eq!(open_button(&tax), Some(Service::Tax));

        let exchange = Text::parse_str("dialog_name|tax\nexchange_tokens|\n");
        assert!(wants_exchange(&exchange));

        let spray = Text::parse_str("dialog_name|tax\nexchange_spray|\n");
        assert!(wants_spray_exchange(&spray));

        let buy_spray = Text::parse_str("dialog_name|spray\nbuy_spray|\n");
        assert!(wants_buy_spray(&buy_spray));

        let cheats = Text::parse_str("dialog_name|cheats\ncheat_auto_farm|\n");
        assert_eq!(cheat_button(&cheats), Some("auto_farm"));
    }

    #[test]
    fn explicit_button_wins_over_other_fields() {
        let close = Text::parse_str("buttonClicked|close\nbuy_bluegem|\nconvert_whitegem|\n");
        assert!(lock_button(&close).is_none());
        assert!(!wants_white_conversion(&close));
        let buy = Text::parse_str("buttonClicked|buy_bluegem\n");
        assert_eq!(lock_button(&buy).map(|spec| spec.kind), Some(LockKind::BlueGem));
    }
}
