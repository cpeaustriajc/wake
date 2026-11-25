//! Standard base64 with padding, for terminal output on the wire.

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const PAD: u8 = b'=';
const GROUP_BYTES: usize = 3;
const GROUP_SYMBOLS: usize = 4;
const SYMBOL_BITS: u32 = 6;
const SYMBOL_MASK: u32 = 0x3f;

fn symbol(six_bits: u32) -> char {
    let index = usize::try_from(six_bits & SYMBOL_MASK).unwrap_or(0);
    char::from(ALPHABET[index])
}

pub(crate) fn encode(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(bytes.len().div_ceil(GROUP_BYTES) * GROUP_SYMBOLS);
    for group in bytes.chunks(GROUP_BYTES) {
        let byte = |index: usize| u32::from(group.get(index).copied().unwrap_or(0));
        let bits = (byte(0) << 16) | (byte(1) << 8) | byte(2);
        for (position, shift) in [18, 12, 6, 0].into_iter().enumerate() {
            if position <= group.len() {
                text.push(symbol(bits >> shift));
            } else {
                text.push(char::from(PAD));
            }
        }
    }
    text
}

fn value(letter: u8) -> Option<u32> {
    let index = ALPHABET.iter().position(|&known| known == letter)?;
    u32::try_from(index).ok()
}

pub(crate) fn decode(text: &str) -> Option<Vec<u8>> {
    let symbols = text.as_bytes();
    if !symbols.len().is_multiple_of(GROUP_SYMBOLS) {
        return None;
    }
    let groups = symbols.len() / GROUP_SYMBOLS;
    let mut bytes = Vec::with_capacity(groups * GROUP_BYTES);
    for (index, group) in symbols.chunks(GROUP_SYMBOLS).enumerate() {
        let padding = group.iter().rev().take_while(|&&last| last == PAD).count();
        if padding >= GROUP_BYTES || (padding > 0 && index + 1 != groups) {
            return None;
        }
        let mut bits = 0_u32;
        for position in 0..GROUP_SYMBOLS {
            let six_bits = match group.get(position).copied() {
                Some(letter) if position < GROUP_SYMBOLS - padding => value(letter)?,
                _ => 0,
            };
            bits = (bits << SYMBOL_BITS) | six_bits;
        }
        let [_, first, second, third] = bits.to_be_bytes();
        bytes.extend(
            [first, second, third]
                .into_iter()
                .take(GROUP_BYTES - padding),
        );
    }
    Some(bytes)
}
