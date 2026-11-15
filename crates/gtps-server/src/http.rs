use crate::{config::Config, logging};
use gtps_auth::{decode_login_token, encode_login_token, AuthStore};
use std::collections::HashMap;
use std::io::Read;
use std::time::{Duration, Instant};
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

pub const END_MARKER: &str = "RTENDMARKERBS1001";

pub fn spawn(config: &Config) -> gtps_auth::Result<()> {
    let server = Server::http(config.http_bind)?;
    let auth = AuthStore::open(&config.auth_path)?;
    let config = config.clone();
    logging::info(format!("http listening on {}", config.http_bind));
    std::thread::spawn(move || {
        let mut limits = HashMap::new();
        for request in server.incoming_requests() {
            handle(request, &config, &auth, &mut limits);
        }
    });
    Ok(())
}

fn reply(request: Request, status: u16, content: &str, body: String, location: Option<String>) {
    let mut response = Response::from_string(body).with_status_code(StatusCode(status));
    for (name, value) in [
        ("Content-Type", content),
        ("Cache-Control", "no-store"),
        ("Referrer-Policy", "no-referrer"),
    ] {
        response.add_header(Header::from_bytes(name, value).expect("static header"));
    }
    if let Some(location) = location {
        if let Ok(header) = Header::from_bytes("Location", location) {
            response.add_header(header);
        }
    }
    let _ = request.respond(response);
}

fn portal() -> String {
    "<!doctype html><html><meta name=viewport content='width=device-width,initial-scale=1'><title>GrowID</title><body><h1>GrowID</h1><form method=post action='/player/growid/login/validate'><label>GrowID <input name=growId required maxlength=18 autocomplete=username></label><label>Password <input name=password type=password required maxlength=128 autocomplete=current-password></label><button>Login</button></form><h2>Create account</h2><form method=post action='/player/growid/register/validate'><label>GrowID <input name=growId required minlength=3 maxlength=18></label><label>Password <input name=password type=password required minlength=10 maxlength=128></label><label>Confirm password <input name=confirmPassword type=password required></label><button>Register</button></form></body></html>".into()
}

fn payload(token: &str) -> String {
    serde_json::json!({"status":"success","message":"Account Validated.","token":token,"url":"","accountType":"growtopia","accountAge":2}).to_string()
}

fn handle(
    mut request: Request,
    config: &Config,
    auth: &AuthStore,
    limits: &mut HashMap<std::net::IpAddr, (Instant, u32)>,
) {
    let url = request.url().to_string();
    let (path, query) = url.split_once('?').unwrap_or((&url, ""));
    if path == "/health" {
        reply(
            request,
            200,
            "application/json",
            "{\"status\":\"ok\"}".into(),
            None,
        );
        return;
    }
    if path == "/growtopia/server_data.php" || path == "/server_data.php" {
        reply(request, 200, "text/plain", server_data(config), None);
        return;
    }
    if request.body_length().is_some_and(|size| size > 16384) {
        reply(request, 413, "text/plain", "Request too large".into(), None);
        return;
    }
    limits.retain(|_, (time, _)| time.elapsed() < Duration::from_secs(60));
    if let Some(address) = request.remote_addr() {
        let entry = limits.entry(address.ip()).or_insert((Instant::now(), 0));
        entry.1 += 1;
        if entry.1 > 30 {
            reply(request, 429, "text/plain", "Try again later".into(), None);
            return;
        }
    }
    let json = request.headers().iter().any(|h| {
        (h.field.equiv("Content-Type") || h.field.equiv("Accept"))
            && h.value.as_str().contains("application/json")
    });
    let mut body = String::new();
    if request
        .as_reader()
        .take(16385)
        .read_to_string(&mut body)
        .is_err()
        || body.len() > 16384
    {
        reply(request, 400, "text/plain", "Invalid body".into(), None);
        return;
    }
    let mut fields: HashMap<String, String> = serde_urlencoded::from_str(query).unwrap_or_default();
    let parsed: HashMap<String, String> = if body.trim_start().starts_with('{') {
        serde_json::from_str(&body).unwrap_or_default()
    } else {
        serde_urlencoded::from_str(&body).unwrap_or_default()
    };
    fields.extend(parsed);
    let client = gtps_proto::Text::parse_str(
        fields
            .get("clientData")
            .or_else(|| fields.get("data"))
            .map_or(body.as_str(), String::as_str),
    );
    let name = fields
        .get("growId")
        .or_else(|| fields.get("tankIDName"))
        .map(String::as_str)
        .or_else(|| client.get("tankIDName"))
        .unwrap_or("");
    let password = fields
        .get("password")
        .or_else(|| fields.get("tankIDPass"))
        .map(String::as_str)
        .or_else(|| client.get("tankIDPass"))
        .unwrap_or("");
    if let Some(encoded) = path.strip_prefix("/player/growid/validate/checktoken/") {
        let decoded: HashMap<String, String> =
            serde_urlencoded::from_str(&format!("token={}", encoded.replace('+', "%2B")))
                .unwrap_or_default();
        let encoded = decoded.get("token").map_or(encoded, String::as_str);
        let valid = decode_login_token(encoded).and_then(|token| auth.verify_token(&token, false));
        if valid.is_ok() {
            reply(request, 200, "application/json", payload(encoded), None);
        } else {
            reply(
                request,
                401,
                "text/plain",
                "Invalid or expired token".into(),
                None,
            );
        }
        return;
    }
    let register = matches!(path, "/register" | "/player/growid/register/validate");
    let login = matches!(path, "/login" | "/player/growid/login/validate");
    let check = path == "/player/growid/checktoken";
    let attempt = (register || login) && request.method() == &Method::Post || check;
    if attempt {
        let result = if check && name.is_empty() {
            fields
                .get("refreshToken")
                .or_else(|| fields.get("token"))
                .or_else(|| fields.get("_token"))
                .ok_or_else(|| -> Box<dyn std::error::Error + Send + Sync> {
                    "Missing token".into()
                })
                .and_then(|value| decode_login_token(value).or_else(|_| Ok(value.clone())))
                .and_then(|token| auth.verify_token(&token, false))
        } else if register {
            if fields.get("confirmPassword").map(String::as_str) != Some(password) {
                Err("Passwords do not match".into())
            } else {
                auth.register(name, password)
            }
        } else {
            auth.login(name, password)
        };
        let result = result.and_then(|account| {
            if auth.banned(account.id)? {
                return Err("Account banned".into());
            }
            Ok(encode_login_token(&auth.issue(&account)?, &account.name))
        });
        match result {
            Ok(token) => {
                if json { reply(request, 200, "application/json", payload(&token), None); }
                else {
                    let escaped = serde_urlencoded::to_string([("token", &token)]).expect("string pair");
                    reply(request, 302, "text/plain", String::new(), Some(format!("/player/growid/validate/checktoken/{}", &escaped[6..])));
                }
            }
            Err(_) if check => reply(request, 200, "text/html", portal(), None),
            Err(_) => reply(request, 401, "application/json", "{\"status\":\"error\",\"message\":\"Invalid credentials or registration details\"}".into(), None),
        }
    } else if register || login || matches!(path, "/" | "/dashboard" | "/player/login/dashboard") {
        reply(request, 200, "text/html", portal(), None);
    } else {
        reply(request, 404, "text/plain", "Not found".into(), None);
    }
}

pub fn server_data(config: &Config) -> String {
    format!(
        "server|{}\nport|{}\ntype|1\nmeta|localhost\nloginurl|{}\n{END_MARKER}\n",
        config.public_host,
        config.game_bind.port(),
        config.login_url
    )
}
