//! Single-user authentication.
//!
//! The API is gated behind bearer-token sessions. A password is set either via the
//! `APP_PASSWORD` environment variable (recommended — it survives DB resets on free
//! hosting) or via the `/auth/setup` endpoint (stored hashed in the database).
//! Passwords are never stored in plaintext: they are hashed with PBKDF2-HMAC-SHA256.
//!
//! Sessions are kept in memory, so a server restart invalidates all tokens (users
//! simply log in again). This is intentionally simple and appropriate for a
//! single-user tool; multi-user auth would need a proper identity provider.

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use ring::pbkdf2;
use ring::rand::{SecureRandom, SystemRandom};
use std::collections::HashMap;
use std::num::NonZeroU32;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use crate::db::Db;

const CRED_LEN: usize = 32;
const ITERATIONS: u32 = 100_000;
const SESSION_TTL: Duration = Duration::from_secs(7 * 24 * 60 * 60); // 7 days

static SESSIONS: OnceLock<Mutex<HashMap<String, Instant>>> = OnceLock::new();

fn sessions() -> &'static Mutex<HashMap<String, Instant>> {
    SESSIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn random_bytes(n: usize) -> Vec<u8> {
    let mut buf = vec![0u8; n];
    if SystemRandom::new().fill(&mut buf).is_err() {
        buf.clear();
        buf.extend(std::iter::repeat(0u8).take(n));
    }
    buf
}

/// Hash a password to a self-describing `base64(salt)$base64(hash)` string.
pub fn hash_password(password: &str) -> String {
    let salt = random_bytes(16);
    let mut out = [0u8; CRED_LEN];
    pbkdf2::derive(
        pbkdf2::PBKDF2_HMAC_SHA256,
        NonZeroU32::new(ITERATIONS).unwrap(),
        &salt,
        password.as_bytes(),
        &mut out,
    );
    format!("{}${}", B64.encode(salt), B64.encode(out))
}

/// Constant-time check of a password against a stored hash string.
pub fn verify_password(password: &str, stored: &str) -> bool {
    let Some((salt_b64, hash_b64)) = stored.split_once('$') else {
        return false;
    };
    let Ok(salt) = B64.decode(salt_b64) else {
        return false;
    };
    let Ok(expected) = B64.decode(hash_b64) else {
        return false;
    };
    let mut out = [0u8; CRED_LEN];
    pbkdf2::derive(
        pbkdf2::PBKDF2_HMAC_SHA256,
        NonZeroU32::new(ITERATIONS).unwrap(),
        &salt,
        password.as_bytes(),
        &mut out,
    );
    constant_time_eq(&out, &expected)
}

/// Constant-time byte comparison (no short-circuit on the first mismatch).
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Whether a password is configured (env var preferred, then DB hash).
pub fn is_password_set(db: &Db) -> bool {
    if let Ok(p) = std::env::var("APP_PASSWORD") {
        if !p.trim().is_empty() {
            return true;
        }
    }
    db.get_setting("auth.password_hash")
        .map(|h| !h.is_empty())
        .unwrap_or(false)
}

/// Verify a login attempt against the configured password (env var or DB hash).
pub fn authenticate(db: &Db, password: &str) -> bool {
    if let Ok(p) = std::env::var("APP_PASSWORD") {
        if !p.trim().is_empty() {
            return verify_password(password, &hash_password(&p));
        }
    }
    match db.get_setting("auth.password_hash") {
        Some(h) if !h.is_empty() => verify_password(password, &h),
        _ => false,
    }
}

/// Set a password (hashed) in the database.
pub fn set_password(db: &Db, password: &str) -> Result<(), rusqlite::Error> {
    if password.len() < 8 {
        return Err(rusqlite::Error::InvalidQuery); // caller maps to a friendly message
    }
    db.set_setting("auth.password_hash", &hash_password(password))
}

/// Create a new session token for an authenticated user.
pub fn create_session() -> String {
    let token = B64.encode(random_bytes(32));
    let now = Instant::now();
    let mut map = sessions().lock().unwrap_or_else(|p| p.into_inner());
    map.insert(token.clone(), now);
    token
}

/// Whether the given bearer token is a currently valid session.
pub fn is_valid_token(token: &str) -> bool {
    let mut map = sessions().lock().unwrap_or_else(|p| p.into_inner());
    let now = Instant::now();
    match map.get(token) {
        Some(created) if now.duration_since(*created) <= SESSION_TTL => true,
        Some(_) => {
            map.remove(token);
            false
        }
        None => false,
    }
}

/// Invalidate a session (logout).
pub fn revoke_session(token: &str) {
    let mut map = sessions().lock().unwrap_or_else(|p| p.into_inner());
    map.remove(token);
}

/// Count of live sessions (for diagnostics / tests).
#[cfg(test)]
pub fn session_count() -> usize {
    sessions()
        .lock()
        .map(|m| m.len())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashes_and_verifies_password() {
        let h = hash_password("correct horse");
        // Not stored in plaintext.
        assert!(!h.contains("correct horse"));
        assert!(verify_password("correct horse", &h));
        assert!(!verify_password("wrong", &h));
    }

    #[test]
    fn verify_rejects_malformed() {
        assert!(!verify_password("x", "not-a-hash"));
        assert!(!verify_password("x", "salt_only$"));
    }

    #[test]
    fn constant_time_comparison_works() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"ab", b"abc"));
        assert!(!constant_time_eq(&[0u8; 0], b"x"));
    }

    #[test]
    fn sessions_create_and_validate() {
        let t1 = create_session();
        let t2 = create_session();
        assert!(is_valid_token(&t1));
        assert!(is_valid_token(&t2));
        assert!(!is_valid_token("bogus-token"));
        revoke_session(&t1);
        assert!(!is_valid_token(&t1));
        assert!(is_valid_token(&t2));
    }
}
