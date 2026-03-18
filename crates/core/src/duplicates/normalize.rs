use xxhash_rust::xxh3::xxh3_64;

use super::tokenize::{SourceToken, TokenKind};
use super::types::DetectionMode;

/// A token with a precomputed hash for use in the detection engine.
#[derive(Debug, Clone)]
pub struct HashedToken {
    /// Hash of the normalized token.
    pub hash: u64,
    /// Index of this token in the original (pre-normalization) token sequence.
    pub original_index: usize,
}

/// Normalize and hash a token sequence according to the detection mode.
///
/// Returns a vector of `HashedToken` values ready for the Rabin-Karp sliding window.
/// Tokens that should be skipped (based on mode) are excluded from the output.
pub fn normalize_and_hash(tokens: &[SourceToken], mode: DetectionMode) -> Vec<HashedToken> {
    let mut result = Vec::with_capacity(tokens.len());

    for (i, token) in tokens.iter().enumerate() {
        let normalized = normalize_token(&token.kind, mode);
        if let Some(hash) = normalized {
            result.push(HashedToken {
                hash,
                original_index: i,
            });
        }
    }

    result
}

/// Normalize a single token and compute its hash.
/// Returns `None` if the token should be skipped in the given mode.
fn normalize_token(kind: &TokenKind, mode: DetectionMode) -> Option<u64> {
    match mode {
        DetectionMode::Strict | DetectionMode::Mild => Some(hash_token_strict(kind)),
        DetectionMode::Weak => Some(hash_token_weak(kind)),
        DetectionMode::Semantic => Some(hash_token_semantic(kind)),
    }
}

/// Hash a token preserving its full identity (strict/mild/weak modes).
fn hash_token_strict(kind: &TokenKind) -> u64 {
    match kind {
        TokenKind::Keyword(kw) => hash_bytes(&[0, *kw as u8]),
        TokenKind::Identifier(name) => {
            let mut buf = vec![1];
            buf.extend_from_slice(name.as_bytes());
            hash_bytes(&buf)
        }
        TokenKind::StringLiteral(val) => {
            let mut buf = vec![2];
            buf.extend_from_slice(val.as_bytes());
            hash_bytes(&buf)
        }
        TokenKind::NumericLiteral(val) => {
            let mut buf = vec![3];
            buf.extend_from_slice(val.as_bytes());
            hash_bytes(&buf)
        }
        TokenKind::BooleanLiteral(val) => hash_bytes(&[4, *val as u8]),
        TokenKind::NullLiteral => hash_bytes(&[5]),
        TokenKind::TemplateLiteral => hash_bytes(&[6]),
        TokenKind::RegExpLiteral => hash_bytes(&[7]),
        TokenKind::Operator(op) => hash_bytes(&[8, *op as u8]),
        TokenKind::Punctuation(p) => hash_bytes(&[9, *p as u8]),
    }
}

/// Hash a token with string literals blinded (weak mode).
fn hash_token_weak(kind: &TokenKind) -> u64 {
    match kind {
        // Blind string literals only — keep identifiers and numeric literals
        TokenKind::StringLiteral(_) => hash_bytes(&[2, 0]),
        other => hash_token_strict(other),
    }
}

/// Hash a token with identifiers and literals blinded (semantic mode).
fn hash_token_semantic(kind: &TokenKind) -> u64 {
    match kind {
        TokenKind::Keyword(kw) => hash_bytes(&[0, *kw as u8]),
        // All identifiers map to the same hash
        TokenKind::Identifier(_) => hash_bytes(&[1, 0]),
        // All string literals map to the same hash
        TokenKind::StringLiteral(_) => hash_bytes(&[2, 0]),
        // All numeric literals map to the same hash
        TokenKind::NumericLiteral(_) => hash_bytes(&[3, 0]),
        // Booleans are kept as-is (structurally significant)
        TokenKind::BooleanLiteral(val) => hash_bytes(&[4, *val as u8]),
        TokenKind::NullLiteral => hash_bytes(&[5]),
        TokenKind::TemplateLiteral => hash_bytes(&[6]),
        TokenKind::RegExpLiteral => hash_bytes(&[7]),
        TokenKind::Operator(op) => hash_bytes(&[8, *op as u8]),
        TokenKind::Punctuation(p) => hash_bytes(&[9, *p as u8]),
    }
}

/// Hash a byte slice using xxh3.
fn hash_bytes(data: &[u8]) -> u64 {
    xxh3_64(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::duplicates::tokenize::{KeywordType, OperatorType, PunctuationType};
    use oxc_span::Span;

    fn make_token(kind: TokenKind) -> SourceToken {
        SourceToken {
            kind,
            span: Span::new(0, 0),
        }
    }

    #[test]
    fn strict_mode_preserves_identifiers() {
        let tokens = vec![
            make_token(TokenKind::Identifier("foo".to_string())),
            make_token(TokenKind::Identifier("bar".to_string())),
        ];

        let hashed = normalize_and_hash(&tokens, DetectionMode::Strict);
        assert_eq!(hashed.len(), 2);
        // Different identifiers should have different hashes in strict mode
        assert_ne!(hashed[0].hash, hashed[1].hash);
    }

    #[test]
    fn semantic_mode_blinds_identifiers() {
        let tokens = vec![
            make_token(TokenKind::Identifier("foo".to_string())),
            make_token(TokenKind::Identifier("bar".to_string())),
        ];

        let hashed = normalize_and_hash(&tokens, DetectionMode::Semantic);
        assert_eq!(hashed.len(), 2);
        // Different identifiers should have the SAME hash in semantic mode
        assert_eq!(hashed[0].hash, hashed[1].hash);
    }

    #[test]
    fn semantic_mode_blinds_string_literals() {
        let tokens = vec![
            make_token(TokenKind::StringLiteral("hello".to_string())),
            make_token(TokenKind::StringLiteral("world".to_string())),
        ];

        let hashed = normalize_and_hash(&tokens, DetectionMode::Semantic);
        assert_eq!(hashed.len(), 2);
        assert_eq!(hashed[0].hash, hashed[1].hash);
    }

    #[test]
    fn semantic_mode_blinds_numeric_literals() {
        let tokens = vec![
            make_token(TokenKind::NumericLiteral("42".to_string())),
            make_token(TokenKind::NumericLiteral("99".to_string())),
        ];

        let hashed = normalize_and_hash(&tokens, DetectionMode::Semantic);
        assert_eq!(hashed.len(), 2);
        assert_eq!(hashed[0].hash, hashed[1].hash);
    }

    #[test]
    fn semantic_mode_preserves_booleans() {
        let tokens = vec![
            make_token(TokenKind::BooleanLiteral(true)),
            make_token(TokenKind::BooleanLiteral(false)),
        ];

        let hashed = normalize_and_hash(&tokens, DetectionMode::Semantic);
        assert_eq!(hashed.len(), 2);
        assert_ne!(hashed[0].hash, hashed[1].hash);
    }

    #[test]
    fn semantic_mode_preserves_keywords() {
        let tokens = vec![
            make_token(TokenKind::Keyword(KeywordType::If)),
            make_token(TokenKind::Keyword(KeywordType::While)),
        ];

        let hashed = normalize_and_hash(&tokens, DetectionMode::Semantic);
        assert_eq!(hashed.len(), 2);
        assert_ne!(hashed[0].hash, hashed[1].hash);
    }

    #[test]
    fn preserves_original_indices() {
        let tokens = vec![
            make_token(TokenKind::Keyword(KeywordType::Const)),
            make_token(TokenKind::Identifier("x".to_string())),
            make_token(TokenKind::Operator(OperatorType::Assign)),
        ];

        let hashed = normalize_and_hash(&tokens, DetectionMode::Mild);
        assert_eq!(hashed.len(), 3);
        assert_eq!(hashed[0].original_index, 0);
        assert_eq!(hashed[1].original_index, 1);
        assert_eq!(hashed[2].original_index, 2);
    }

    #[test]
    fn empty_input_produces_empty_output() {
        let tokens: Vec<SourceToken> = vec![];
        let hashed = normalize_and_hash(&tokens, DetectionMode::Mild);
        assert!(hashed.is_empty());
    }

    #[test]
    fn operators_have_distinct_hashes() {
        let tokens = vec![
            make_token(TokenKind::Operator(OperatorType::Add)),
            make_token(TokenKind::Operator(OperatorType::Sub)),
        ];

        let hashed = normalize_and_hash(&tokens, DetectionMode::Strict);
        assert_ne!(hashed[0].hash, hashed[1].hash);
    }

    #[test]
    fn punctuation_has_distinct_hashes() {
        let tokens = vec![
            make_token(TokenKind::Punctuation(PunctuationType::OpenParen)),
            make_token(TokenKind::Punctuation(PunctuationType::CloseParen)),
        ];

        let hashed = normalize_and_hash(&tokens, DetectionMode::Strict);
        assert_ne!(hashed[0].hash, hashed[1].hash);
    }
}
