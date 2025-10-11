use hmac::{Hmac, Mac};
use sha2::Sha256;
use base64::{engine::general_purpose, Engine as _};

pub fn verify_webhook_sig(raw_body: &[u8], signature_b64: Option<&str>, secret: &str) -> bool {
    let Some(sig) = signature_b64 else { return false; };
    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(raw_body);
    let calc = mac.finalize().into_bytes();
    let calc_b64 = general_purpose::STANDARD.encode(calc);
    calc_b64.eq(sig) || calc_b64.trim_end_matches('=').eq(sig.trim_end_matches('='))
}
