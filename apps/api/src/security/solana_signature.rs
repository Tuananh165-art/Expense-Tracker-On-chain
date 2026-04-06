use bs58;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};

pub fn verify_signature_base58(
    wallet_address_base58: &str,
    signature_base58: &str,
    message: &str,
) -> bool {
    let pubkey_bytes = match bs58::decode(wallet_address_base58).into_vec() {
        Ok(v) if v.len() == 32 => v,
        _ => return false,
    };

    let sig_bytes = match bs58::decode(signature_base58).into_vec() {
        Ok(v) if v.len() == 64 => v,
        _ => return false,
    };

    let pubkey_arr: [u8; 32] = match pubkey_bytes.try_into() {
        Ok(v) => v,
        Err(_) => return false,
    };
    let sig_arr: [u8; 64] = match sig_bytes.try_into() {
        Ok(v) => v,
        Err(_) => return false,
    };

    let verifying_key = match VerifyingKey::from_bytes(&pubkey_arr) {
        Ok(k) => k,
        Err(_) => return false,
    };

    let signature = Signature::from_bytes(&sig_arr);
    verifying_key.verify(message.as_bytes(), &signature).is_ok()
}
