//! Вшитый owner/repo для GitHub Releases (DecodeEmbeddedConfig).
//! Значения закодированы XOR — в UI и пользовательских сообщениях не показываются.

const XOR_KEY: u8 = 0x5C;

/// `Fantomhivec` XOR 0x5C
const OWNER_ENC: &[u8] = &[26, 61, 50, 40, 51, 49, 52, 53, 42, 57, 63];

/// `StatHive` XOR 0x5C
const REPO_ENC: &[u8] = &[15, 40, 61, 40, 20, 53, 42, 57];

fn reveal(enc: &[u8]) -> String {
    enc.iter().map(|b| (b ^ XOR_KEY) as char).collect()
}

pub fn github_owner() -> String {
    reveal(OWNER_ENC)
}

pub fn github_repo() -> String {
    reveal(REPO_ENC)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_owner_repo() {
        assert_eq!(github_owner(), "Fantomhivec");
        assert_eq!(github_repo(), "StatHive");
    }
}
