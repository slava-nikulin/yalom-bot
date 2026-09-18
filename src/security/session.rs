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
use tower_cookies::cookie::time::{
    Duration, OffsetDateTime, format_description::well_known::Rfc3339,
};

#[derive(Debug, thiserror::Error)]
pub enum SessionIssueError {
    #[error("failed to issue session token")]
    Paseto(#[from] pasetors::errors::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum SessionValidationError {
    #[error("invalid session token")]
    InvalidToken(#[from] pasetors::errors::Error),

    #[error("session claims are missing")]
    MissingClaims,

    #[error("session subject is missing")]
    MissingSubject,

    #[error("invalid session subject")]
    InvalidSubject,
}

#[derive(Clone)]
pub struct SessionTokens {
    key: SymmetricKey<V4>,
}

impl SessionTokens {
    pub fn new(session_key: &str) -> Result<Self, SessionIssueError> {
        let hk = Hkdf::<Sha256>::new(None, session_key.as_bytes());
        let mut okm = [0u8; 32];
        hk.expand(b"yalom_bot/paseto-v4-local/session/v1", &mut okm)
            .expect("32 bytes is a valid HKDF-SHA256 output length");

        Ok(Self {
            key: SymmetricKey::<V4>::from(okm.as_slice())?,
        })
    }

    pub fn issue(&self, user_id: i64, ttl: i64) -> Result<String, SessionIssueError> {
        let mut claims = Claims::new()?;

        claims.subject(&user_id.to_string())?;

        let expiration = OffsetDateTime::now_utc()
            .checked_add(Duration::seconds(ttl))
            .expect("time addition should not overflow for a reasonable TTL");
        let expiration = expiration
            .format(&Rfc3339)
            .expect("RFC3339 formatting of a valid timestamp should succeed");
        claims.expiration(&expiration)?;

        let token = local::encrypt(&self.key, &claims, None, None)?;

        Ok(token)
    }

    pub fn validate(&self, auth_token: &str) -> Result<SessionIdentity, SessionValidationError> {
        let untrusted = UntrustedToken::<Local, V4>::try_from(auth_token)?;

        let rules = ClaimsValidationRules::new();

        let trusted = local::decrypt(&self.key, &untrusted, &rules, None, None)?;

        let claims = trusted
            .payload_claims()
            .ok_or(SessionValidationError::MissingClaims)?;

        Ok(SessionIdentity {
            tg_user_id: claims
                .get_claim("sub")
                .ok_or(SessionValidationError::MissingSubject)?
                .as_str()
                .ok_or(SessionValidationError::InvalidSubject)?
                .parse::<i64>()
                .map_err(|_| SessionValidationError::InvalidSubject)?,
        })
    }
}

#[derive(Clone)]
pub struct SessionIdentity {
    pub tg_user_id: i64,
}
