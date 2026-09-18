use hkdf::Hkdf;
use pasetors::{
    Local,
    claims::{Claims, ClaimsValidationRules},
    keys::SymmetricKey,
    local,
    token::UntrustedToken,
    version4::V4,
};
use rand::RngExt;
use sha2::Sha256;

const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_-";

pub fn generate_secret_token(len: u16) -> String {
    let mut rng = rand::rng();

    (0..len)
        .map(|_| CHARSET[rng.random_range(0..CHARSET.len())] as char)
        .collect()
}

#[derive(Clone)]
pub struct SessionTokens {
    key: SymmetricKey<V4>,
}

impl SessionTokens {
    pub fn new(tg_token: &str) -> anyhow::Result<Self> {
        let hk = Hkdf::<Sha256>::new(None, tg_token.as_bytes());
        let mut okm = [0u8; 32];
        hk.expand(b"yalob_bot/paseto-v4-local/session/v1", &mut okm)
            .expect("32 bytes is a valid HKDF-SHA256 output length");

        Ok(Self {
            key: SymmetricKey::<V4>::from(tg_token.as_bytes())?,
        })
    }

    pub fn build_paseto_token(&self, user_id: i64) -> anyhow::Result<String> {
        let mut claims = Claims::new()?;

        claims.subject(&user_id.to_string())?;

        let token = local::encrypt(&self.key, &claims, None, None)?;

        Ok(token)
    }

    pub fn validate_paseto_token(&self, auth_token: &str) -> anyhow::Result<()> {
        let untrusted = UntrustedToken::<Local, V4>::try_from(auth_token)?;

        let rules = ClaimsValidationRules::new();

        let trusted = local::decrypt(&self.key, &untrusted, &rules, None, None)?;

        let claims = trusted.payload_claims().unwrap();

        println!("{:?}", claims.get_claim("sub"));

        Ok(())
    }
}
