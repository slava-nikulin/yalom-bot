use core::time;

use hkdf::Hkdf;
use pasetors::{
    Local,
    claims::{Claims, ClaimsValidationRules},
    keys::SymmetricKey,
    local,
    token::UntrustedToken,
    version4::V4,
};
use sha2::Sha256;

pub const SESSION_TTL_SECONDS: i64 = 30 * 60;

#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error(transparent)]
    Paseto(#[from] pasetors::errors::Error),

    #[error("invalid session token claims")]
    InvalidClaims,
}

#[derive(Clone)]
pub struct SessionIdentity {
    pub tg_user_id: i64,
}

#[derive(Clone)]
pub struct Sessions {
    key: SymmetricKey<V4>,
}

impl Sessions {
    pub fn new(session_key: &str) -> Self {
        let hk = Hkdf::<Sha256>::new(None, session_key.as_bytes());
        let mut okm = [0u8; 32];
        hk.expand(b"yalom_bot/paseto-v4-local/session/v1", &mut okm)
            .expect("32 bytes is a valid HKDF-SHA256 output length");

        Self {
            key: SymmetricKey::<V4>::from(&okm).expect("PASETO v4 local keys are exactly 32 bytes"),
        }
    }

    pub fn issue(&self, tg_user_id: i64) -> Result<String, SessionError> {
        let mut claims =
            Claims::new_expires_in(&time::Duration::from_secs(SESSION_TTL_SECONDS as u64))?;

        claims
            .subject(&tg_user_id.to_string())
            .expect("i64 string representation is never empty");

        let token = local::encrypt(&self.key, &claims, None, None)?;

        Ok(token)
    }

    pub fn validate(&self, auth_token: &str) -> Result<SessionIdentity, SessionError> {
        let untrusted = UntrustedToken::<Local, V4>::try_from(auth_token)?;

        let rules = ClaimsValidationRules::new();

        let trusted = local::decrypt(&self.key, &untrusted, &rules, None, None)?;

        let tg_user_id = trusted
            .payload_claims()
            .and_then(|claims| claims.get_claim("sub"))
            .and_then(|subject| subject.as_str())
            .and_then(|subject| subject.parse::<i64>().ok())
            .ok_or(SessionError::InvalidClaims)?;

        Ok(SessionIdentity { tg_user_id })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SESSION_KEY: &str = "test-session-key";

    #[test]
    fn correct_validate() {
        let session_tokens = Sessions::new(SESSION_KEY);

        let issued_token = session_tokens.issue(123).unwrap();
        let ident = session_tokens.validate(&issued_token).unwrap();

        assert_eq!(ident.tg_user_id, 123);
    }

    #[test]
    fn incorrect_token_rejected_on_validate() {
        let session_tokens = Sessions::new(SESSION_KEY);

        let res = session_tokens.validate("wrong token");

        assert!(res.is_err());
    }

    #[test]
    fn token_from_wrong_key_rejected_on_validate() {
        let session_tokens_a = Sessions::new("a key");
        let issued_token_a = session_tokens_a.issue(123).unwrap();

        let session_tokens_b = Sessions::new("b key");

        let res = session_tokens_b.validate(&issued_token_a);

        assert!(res.is_err());
    }
}
