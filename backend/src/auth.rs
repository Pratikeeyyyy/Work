//! Multi-user authentication.
//!
//! The API is gated behind bearer-token sessions. Each registered user has an
//! account (username + password) stored in the central `users` table, and their
//! data lives in an isolated per-user database. Passwords are never stored in
//! plaintext: they are hashed with PBKDF2-HMAC-SHA256.
//!
//! Sessions are kept in memory and bound to the username that logged in, so a
//! server restart invalidates all tokens (users simply log in again).

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use ring::pbkdf2;
use ring::rand::{SecureRandom, SystemRandom};
use std::collections::HashMap;
use std::num::NonZeroU32;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

const CRED_LEN: usize = 32;
const ITERATIONS: u32 = 100_000;
const SESSION_TTL: Duration = Duration::from_secs(7 * 24 * 60 * 60); // 7 days

/// Sessions map: bearer token -> (username, created_at).
static SESSIONS: OnceLock<Mutex<HashMap<String, (String, Instant)>>> = OnceLock::new();

fn sessions() -> &'static Mutex<HashMap<String, (String, Instant)>> {
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

/// Create a new session token bound to a logged-in user.
pub fn create_session(username: &str) -> String {
    let token = B64.encode(random_bytes(32));
    let now = Instant::now();
    let mut map = sessions().lock().unwrap_or_else(|p| p.into_inner());
    map.insert(token.clone(), (username.to_string(), now));
    token
}

/// If the bearer token is a current valid session, return the username it
/// belongs to. Expired sessions are lazily evicted.
pub fn username_for_token(token: &str) -> Option<String> {
    let mut map = sessions().lock().unwrap_or_else(|p| p.into_inner());
    let now = Instant::now();
    match map.get(token) {
        Some((username, created)) if now.duration_since(*created) <= SESSION_TTL => {
            Some(username.clone())
        }
        Some(_) => {
            map.remove(token);
            None
        }
        None => None,
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
        let t1 = create_session("alice");
        let t2 = create_session("bob");
        assert!(username_for_token(&t1).is_some());
        assert!(username_for_token(&t2).is_some());
        assert!(username_for_token("bogus-token").is_none());
        // Session is bound to the user who logged in.
        assert_eq!(username_for_token(&t1).as_deref(), Some("alice"));
        assert_eq!(username_for_token(&t2).as_deref(), Some("bob"));
        revoke_session(&t1);
        assert!(username_for_token(&t1).is_none());
        assert!(username_for_token(&t2).is_some());
    }
}
