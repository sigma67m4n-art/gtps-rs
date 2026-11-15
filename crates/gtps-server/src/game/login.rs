#[cfg(test)]
use gtps_proto::Text;
use gtps_proto::{Variant, VariantList};

#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginRequest {
    pub grow_id: Option<String>,
    pub password: String,
    pub requested_name: String,
    pub country: String,
    pub protocol: u32,
    pub game_version: String,
    pub login_mode: u32,
}

#[cfg(test)]
impl LoginRequest {
    #[must_use]
    pub fn parse(text: &Text) -> Option<Self> {
        let grow_id = text.get("tankIDName").unwrap_or("").trim().to_string();
        let requested_name = text.get("requestedName").unwrap_or("").trim().to_string();

        if grow_id.is_empty() && requested_name.is_empty() {
            return None;
        }

        Some(Self {
            grow_id: (!grow_id.is_empty()).then_some(grow_id),
            password: text.get("tankIDPass").unwrap_or("").to_string(),
            requested_name,
            country: text.get("country").unwrap_or("us").to_string(),
            protocol: text.get_i64("protocol").unwrap_or(0) as u32,
            game_version: text.get("game_version").unwrap_or("").to_string(),
            login_mode: text.get_i64("lmode").unwrap_or(0) as u32,
        })
    }

    #[must_use]
    pub fn display_name(&self) -> &str {
        match &self.grow_id {
            Some(grow_id) => grow_id.as_str(),
            None => self.requested_name.as_str(),
        }
    }

    #[must_use]
    pub fn is_transfer(&self) -> bool {
        self.login_mode == 2
    }
}

#[must_use]
pub fn super_main_start(items_hash: u32, cdn_host: &str) -> VariantList {
    VariantList::new("OnSuperMainStart")
        .arg(Variant::Uint(items_hash))
        .arg(Variant::String(cdn_host.to_string()))
        .arg(Variant::String(String::new()))
}

#[must_use]
pub fn world_select_menu(server_name: &str, motd: &str) -> VariantList {
    VariantList::new("OnRequestWorldSelectMenu").arg(Variant::String(format!(
        "add_filter|Categories\nadd_filter|{server_name}\nadd_filter|Recent Worlds\nadd_filter|Top Worlds\nadd_filter|Random\nadd_filter|{motd}\n"
    )))
}

#[must_use]
pub fn console_message(message: &str) -> VariantList {
    VariantList::new("OnConsoleMessage").arg(Variant::String(message.to_string()))
}

#[must_use]
pub fn failed_to_enter() -> VariantList {
    VariantList::new("OnFailedToEnterWorld")
}

#[cfg(test)]
mod tests {
    use super::LoginRequest;
    use gtps_proto::Text;

    #[test]
    fn guest_login_parses() {
        let text = Text::parse_str("requestedName|Guest\nprotocol|225\ncountry|id\nlmode|0\n");
        let request = LoginRequest::parse(&text).expect("login request");
        assert_eq!(request.display_name(), "Guest");
        assert!(request.grow_id.is_none());
        assert_eq!(request.protocol, 225);
        assert_eq!(request.country, "id");
        assert!(!request.is_transfer());
    }

    #[test]
    fn grow_id_login_parses() {
        let text = Text::parse_str("tankIDName|Craig\ntankIDPass|secret\nlmode|2\n");
        let request = LoginRequest::parse(&text).expect("login request");
        assert_eq!(request.display_name(), "Craig");
        assert_eq!(request.grow_id.as_deref(), Some("Craig"));
        assert!(request.is_transfer());
    }
}
