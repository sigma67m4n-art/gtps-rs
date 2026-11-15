use argon2::{password_hash::SaltString, Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use base64::{engine::general_purpose::STANDARD, Engine};
use rand_core::{OsRng, RngCore};
use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::Path;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Debug, Clone)]
pub struct Account {
    pub id: u32,
    pub name: String,
    pub role: u8,
}

pub struct AuthStore {
    db: Connection,
}

pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |value| value.as_secs())
}

pub fn valid_name(name: &str) -> bool {
    (3..=18).contains(&name.len()) && name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
}

fn digest(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}

impl AuthStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let db = Connection::open(path)?;
        db.busy_timeout(std::time::Duration::from_secs(5))?;
        db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;
            CREATE TABLE IF NOT EXISTS accounts(id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE COLLATE NOCASE, password TEXT NOT NULL, role INTEGER NOT NULL DEFAULT 0);
            CREATE TABLE IF NOT EXISTS tokens(hash BLOB PRIMARY KEY, account INTEGER NOT NULL REFERENCES accounts(id), expires INTEGER NOT NULL);
            CREATE TABLE IF NOT EXISTS player_state(account INTEGER PRIMARY KEY REFERENCES accounts(id), data BLOB NOT NULL);")?;
        db.execute_batch("CREATE TABLE IF NOT EXISTS server_state(id INTEGER PRIMARY KEY CHECK(id=1), data BLOB NOT NULL);
            CREATE TABLE IF NOT EXISTS bans(account INTEGER PRIMARY KEY REFERENCES accounts(id));")?;
        Ok(Self { db })
    }

    pub fn register(&self, name: &str, password: &str) -> Result<Account> {
        if !valid_name(name) || !(10..=128).contains(&password.len()) {
            return Err("Use a 3-18 character GrowID and a 10-128 byte password".into());
        }
        let salt = SaltString::generate(&mut OsRng);
        let hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map_err(|_| "password hashing failed")?
            .to_string();
        self.db
            .execute(
                "INSERT INTO accounts(name,password) VALUES (?1,?2)",
                params![name, hash],
            )
            .map_err(|_| "Account registration failed")?;
        Ok(Account {
            id: self.db.last_insert_rowid().try_into()?,
            name: name.to_string(),
            role: 0,
        })
    }

    pub fn login(&self, name: &str, password: &str) -> Result<Account> {
        if !valid_name(name) || password.len() > 128 {
            return Err("Invalid credentials".into());
        }
        let row = self
            .db
            .query_row(
                "SELECT id,name,role,password FROM accounts WHERE name=?1",
                [name],
                |row| {
                    Ok((
                        Account {
                            id: row.get(0)?,
                            name: row.get(1)?,
                            role: row.get(2)?,
                        },
                        row.get::<_, String>(3)?,
                    ))
                },
            )
            .optional()?;
        let Some((account, hash)) = row else {
            return Err("Invalid credentials".into());
        };
        let parsed = PasswordHash::new(&hash).map_err(|_| "Invalid credentials")?;
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .map_err(|_| "Invalid credentials")?;
        Ok(account)
    }

    pub fn issue(&self, account: &Account) -> Result<String> {
        let mut bytes = [0u8; 32];
        OsRng.fill_bytes(&mut bytes);
        let token = bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
        self.db.execute(
            "DELETE FROM tokens WHERE expires<=?1 OR account=?2",
            params![now(), account.id],
        )?;
        self.db.execute(
            "INSERT INTO tokens VALUES (?1,?2,?3)",
            params![digest(&token), account.id, now() + 300],
        )?;
        Ok(token)
    }

    pub fn verify_token(&self, token: &str, consume: bool) -> Result<Account> {
        if token.len() != 64 {
            return Err("Invalid token".into());
        }
        let tx = self.db.unchecked_transaction()?;
        let account = tx.query_row("SELECT a.id,a.name,a.role FROM accounts a JOIN tokens t ON a.id=t.account WHERE t.hash=?1 AND t.expires>?2", params![digest(token), now()], |row| {
            Ok(Account { id: row.get(0)?, name: row.get(1)?, role: row.get(2)? })
        }).map_err(|_| "Invalid or expired token")?;
        if consume {
            tx.execute("DELETE FROM tokens WHERE hash=?1", [digest(token)])?;
        }
        tx.commit()?;
        Ok(account)
    }

    pub fn set_role(&self, id: u32, role: u8) -> Result<()> {
        if role > 2 {
            return Err("Invalid role".into());
        }
        self.db
            .execute("UPDATE accounts SET role=?1 WHERE id=?2", params![role, id])?;
        Ok(())
    }

    pub fn load_state(&self, id: u32) -> Result<Option<Vec<u8>>> {
        Ok(self
            .db
            .query_row(
                "SELECT data FROM player_state WHERE account=?1",
                [id],
                |row| row.get(0),
            )
            .optional()?)
    }

    pub fn save_state(&self, id: u32, data: &[u8]) -> Result<()> {
        self.db.execute("INSERT INTO player_state VALUES (?1,?2) ON CONFLICT(account) DO UPDATE SET data=excluded.data", params![id,data])?;
        Ok(())
    }

    pub fn checkpoint(&self, players: &[(u32, Vec<u8>)], world: &[u8]) -> Result<()> {
        let tx = self.db.unchecked_transaction()?;
        for (id, data) in players {
            tx.execute("INSERT INTO player_state VALUES (?1,?2) ON CONFLICT(account) DO UPDATE SET data=excluded.data", params![id,data])?;
        }
        tx.execute("INSERT INTO server_state VALUES (1,?1) ON CONFLICT(id) DO UPDATE SET data=excluded.data", [world])?;
        tx.commit()?;
        Ok(())
    }

    pub fn world_state(&self) -> Result<Option<Vec<u8>>> {
        Ok(self
            .db
            .query_row("SELECT data FROM server_state WHERE id=1", [], |row| {
                row.get(0)
            })
            .optional()?)
    }

    pub fn banned(&self, id: u32) -> Result<bool> {
        Ok(self.db.query_row(
            "SELECT EXISTS(SELECT 1 FROM bans WHERE account=?1)",
            [id],
            |row| row.get(0),
        )?)
    }

    pub fn ban(&self, id: u32, enabled: bool) -> Result<()> {
        if enabled {
            self.db
                .execute("INSERT OR IGNORE INTO bans VALUES (?1)", [id])?;
        } else {
            self.db.execute("DELETE FROM bans WHERE account=?1", [id])?;
        }
        Ok(())
    }
}

pub fn encode_login_token(token: &str, name: &str) -> String {
    STANDARD.encode(
        serde_urlencoded::to_string([("_token", token), ("username", name), ("type", "login")])
            .expect("string pairs"),
    )
}

pub fn decode_login_token(encoded: &str) -> Result<String> {
    if encoded.len() > 4096 {
        return Err("Token too long".into());
    }
    let raw = STANDARD.decode(encoded)?;
    let fields: HashMap<String, String> = serde_urlencoded::from_bytes(&raw)?;
    fields
        .get("_token")
        .cloned()
        .ok_or_else(|| "Missing token".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn credentials_roles_and_single_use_tokens() {
        let db = AuthStore::open(":memory:").unwrap();
        let account = db.register("Tester", "long-password-123").unwrap();
        assert!(db.register("tester", "another-password").is_err());
        assert!(db.login("Tester", "wrong").is_err());
        assert_eq!(
            db.login("tester", "long-password-123").unwrap().id,
            account.id
        );
        db.set_role(account.id, 2).unwrap();
        let token = db.issue(&account).unwrap();
        let encoded = encode_login_token(&token, "ForgedOwner");
        let decoded = decode_login_token(&encoded).unwrap();
        assert_eq!(db.verify_token(&decoded, true).unwrap().name, "Tester");
        assert!(db.verify_token(&decoded, true).is_err());
        assert_eq!(db.login("Tester", "long-password-123").unwrap().role, 2);
    }

    #[test]
    fn checkpoint_rolls_back_players_and_world_together() {
        let db = AuthStore::open(":memory:").unwrap();
        let account = db.register("Snapshot", "snapshot-password").unwrap();
        db.checkpoint(&[(account.id, b"before".to_vec())], b"world-before")
            .unwrap();
        assert!(db
            .checkpoint(
                &[
                    (account.id, b"after".to_vec()),
                    (u32::MAX, b"invalid".to_vec())
                ],
                b"world-after"
            )
            .is_err());
        assert_eq!(db.load_state(account.id).unwrap().unwrap(), b"before");
        assert_eq!(db.world_state().unwrap().unwrap(), b"world-before");
    }
}
