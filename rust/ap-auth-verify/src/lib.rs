//! Verify incoming signatures in an HTTP server.

use base64::{engine::general_purpose, Engine as _};
use ed25519_dalek::{Signature, VerifyingKey};

pub mod header;

#[derive(Debug)]
pub enum Error {
    Check,
    Header(&'static str),
    Key(&'static str),
    Signature(&'static str),
    Time,
}

/// Verify a request.
pub fn verify<'a, F, G>(method: &str, path: &'a str, body: &[u8], now: u64, get_header: F, get_key: G) -> Result<(), Error>
where F: Fn(&str) -> Option<&'a str>,
    G: FnOnce(u32) -> Result<[u8; 32], &'static str>,
{

    // get the authorization header
    let raw_header = get_header("authorization").unwrap_or("");
    let header = header::AuthHeader::new(raw_header).map_err(Error::Header)?;

    // check for the validity
    if now < header.start || now - header.start > header.duration {
        return Err(Error::Time);
    }

    // get the verifying key
    let pubkey = get_key(header.key).map_err(Error::Key)?;
    let verifykey = VerifyingKey::from_bytes(&pubkey).or(Err(Error::Key("value")))?;

    // decode the signature
    let signature_vec = general_purpose::URL_SAFE_NO_PAD
        .decode(header.sig)
        .or(Err(Error::Signature("base64")))?;
    let sig = Signature::from_slice(&signature_vec).or(Err(Error::Signature("len")))?;


    // produce the message to verify
    let mut message: Vec<u8> = Vec::new();
    message.extend(header.header.as_bytes());
    message.push(b'\n');
    for name in header.add {
        match name.as_str() {
            "-method" => {
                message.extend(method.bytes());
            }
            "-path" => {
                message.extend(path.bytes());
            }
            x => {
                message.extend(get_header(x).unwrap_or("").bytes());
            }
        }
        message.push(b'\n');
    }
    // always add the body
    message.extend(body);


    // verify it
    verifykey
        .verify_strict(&message, &sig)
        .or(Err(Error::Check))
}