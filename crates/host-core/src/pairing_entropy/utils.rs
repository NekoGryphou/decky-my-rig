use super::{
    consts::{ACCEPTED_CODE_RANGE, CODE_RANGE, MAX_CODE_DRAWS},
    messages::FAILURE,
};
use uuid::Uuid;

fn fill(bytes: &mut [u8]) -> Result<(), &'static str> {
    getrandom::getrandom(bytes).map_err(|_| FAILURE)
}

pub fn nonce() -> Result<[u8; 32], &'static str> {
    let mut nonce = [0; 32];
    fill(&mut nonce)?;
    Ok(nonce)
}

pub fn generate() -> Result<(String, [u8; 32], Uuid), &'static str> {
    generate_with(fill)
}

pub(super) fn generate_with(
    mut random: impl FnMut(&mut [u8]) -> Result<(), &'static str>,
) -> Result<(String, [u8; 32], Uuid), &'static str> {
    let mut token = [0; 32];
    random(&mut token)?;
    let mut id = [0; 16];
    random(&mut id)?;
    id[6] = (id[6] & 0x0f) | 0x40;
    id[8] = (id[8] & 0x3f) | 0x80;
    // ! Throw away draws from the incomplete top range, otherwise some codes
    //   would come up more often (modulo bias). A broken random source still
    //   can't loop forever.
    for _ in 0..MAX_CODE_DRAWS {
        let mut bytes = [0; 4];
        random(&mut bytes)?;
        let number = u32::from_le_bytes(bytes);
        if number < ACCEPTED_CODE_RANGE {
            return Ok((
                format!("{:08}", number % CODE_RANGE),
                token,
                Uuid::from_bytes(id),
            ));
        }
    }
    Err(FAILURE)
}
