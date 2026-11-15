#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Text {
    entries: Vec<(String, String)>,
}

fn to_field(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).trim_end_matches('\r').to_string()
}

impl Text {
    #[must_use]
    pub fn parse(input: &[u8]) -> Self {
        let mut entries = Vec::new();

        for line in input.split(|byte| *byte == b'\n') {
            if line.iter().all(|byte| *byte == b'\r') {
                continue;
            }

            let mut parts = line.splitn(2, |byte| *byte == b'|');
            let key = to_field(parts.next().unwrap_or(&[]));
            let value = to_field(parts.next().unwrap_or(&[]));
            entries.push((key, value));
        }

        Self { entries }
    }

    #[must_use]
    pub fn parse_str(input: &str) -> Self {
        Self::parse(input.as_bytes())
    }

    #[must_use]
    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value.as_str())
    }

    #[must_use]
    pub fn get_i64(&self, key: &str) -> Option<i64> {
        self.get(key)?.trim().parse().ok()
    }

    #[must_use]
    pub fn has(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    pub fn set(&mut self, key: &str, value: &str) {
        if let Some(entry) = self.entries.iter_mut().find(|(name, _)| name == key) {
            entry.1 = value.to_string();
        } else {
            self.entries.push((key.to_string(), value.to_string()));
        }
    }

    pub fn remove(&mut self, key: &str) {
        self.entries.retain(|(name, _)| name != key);
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.entries
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str()))
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    #[must_use]
    pub fn to_wire(&self) -> String {
        let mut out = String::new();

        for (key, value) in &self.entries {
            out.push_str(key);
            out.push('|');
            out.push_str(value);
            out.push('\n');
        }

        out
    }
}
