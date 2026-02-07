use sha2::{Digest, Sha256};

/// Generate a hash for a query string
pub fn hash_query(query: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(query.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_query() {
        let query1 = "name:lightning c:red";
        let query2 = "name:lightning c:red";
        let query3 = "name:bolt c:red";

        let hash1 = hash_query(query1);
        let hash2 = hash_query(query2);
        let hash3 = hash_query(query3);

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
        assert_eq!(hash1.len(), 64); // SHA256 produces 64 hex characters
    }
}
