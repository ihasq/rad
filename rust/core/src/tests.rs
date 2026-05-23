#[cfg(test)]
mod tests {
    use crate::crypto::*;
    use crate::types::*;

    #[test]
    fn test_keygen() {
        // Test that keygen produces valid keypairs
        let keypair = generate_keypair();
        // Public key is base64 encoded (32 bytes = 44 chars in base64)
        assert_eq!(keypair.public_key.len(), 44, "Public key should be 44 chars (base64)");
        assert_eq!(keypair.secret_key.len(), 64, "Secret key should be 64 bytes");
    }

    #[test]
    fn test_keygen_unique() {
        // Test that keygen produces unique keys
        let kp1 = generate_keypair();
        let kp2 = generate_keypair();
        assert_ne!(kp1.public_key, kp2.public_key, "Public keys should be unique");
        assert_ne!(kp1.secret_key, kp2.secret_key, "Secret keys should be unique");
    }

    #[test]
    fn test_op_type_serialization() {
        // Test that OpType serializes correctly
        use serde_json;

        let write_op = OpType::Write;
        let json = serde_json::to_string(&write_op).unwrap();
        assert_eq!(json, "\"write\"");

        let reject_op = OpType::Reject;
        let json = serde_json::to_string(&reject_op).unwrap();
        assert_eq!(json, "\"reject\"");
    }

    #[test]
    fn test_op_status_serialization() {
        // Test that OpStatus serializes correctly
        use serde_json;

        let visible = OpStatus::Visible;
        let json = serde_json::to_string(&visible).unwrap();
        assert_eq!(json, "\"visible\"");

        let accepted = OpStatus::Accepted;
        let json = serde_json::to_string(&accepted).unwrap();
        assert_eq!(json, "\"accepted\"");
    }

    #[test]
    fn test_participant_creation() {
        // Test that Participant can be created
        let participant = Participant {
            id: "alice".to_string(),
            public_key: "pubkey123".to_string(),
            display_name: Some("Alice".to_string()),
            joined_at: 1716000000,
        };

        assert_eq!(participant.id, "alice");
        assert_eq!(participant.public_key, "pubkey123");
        assert_eq!(participant.display_name, Some("Alice".to_string()));
    }

    #[test]
    fn test_code_region_creation() {
        // Test that CodeRegion can be created
        let region = CodeRegion {
            id: "main.ts:5-10".to_string(),
            file_path: "main.ts".to_string(),
            start_line: 5,
            end_line: 10,
            owner_id: "alice".to_string(),
        };

        assert_eq!(region.file_path, "main.ts");
        assert_eq!(region.start_line, 5);
        assert_eq!(region.end_line, 10);
    }

    #[test]
    fn test_operation_creation() {
        // Test that Operation can be created
        let op = Operation {
            id: "op-1".to_string(),
            participant_id: "alice".to_string(),
            region_id: "main.ts:5-10".to_string(),
            op_type: OpType::Write,
            content: "const a = 1;".to_string(),
            reason: None,
            signature: "sig123".to_string(),
            timestamp: 1716000000,
            status: OpStatus::Visible,
        };

        assert_eq!(op.id, "op-1");
        assert_eq!(op.op_type, OpType::Write);
        assert_eq!(op.status, OpStatus::Visible);
    }

    #[test]
    fn test_operation_with_reason() {
        // Test that Operation can have a reason (for reject)
        let op = Operation {
            id: "op-2".to_string(),
            participant_id: "bob".to_string(),
            region_id: "file.ts:1-5".to_string(),
            op_type: OpType::Reject,
            content: "".to_string(),
            reason: Some("not good enough".to_string()),
            signature: "sig456".to_string(),
            timestamp: 1716000001,
            status: OpStatus::Visible,
        };

        assert_eq!(op.op_type, OpType::Reject);
        assert_eq!(op.reason, Some("not good enough".to_string()));
    }
}
