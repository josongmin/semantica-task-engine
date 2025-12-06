//! Unit tests for enqueue validation

#[cfg(test)]
mod tests {
    use super::super::*;
    use serde_json::json;

    #[test]
    fn test_validate_queue_name_empty() {
        let req = EnqueueRequest {
            queue: "".to_string(),
            job_type: "test".to_string(),
            subject_key: "key".to_string(),
            payload: json!({}),
            priority: 0,
        };

        let result = validate_request(&req);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    #[test]
    fn test_validate_queue_name_too_long() {
        let req = EnqueueRequest {
            queue: "a".repeat(65),
            job_type: "test".to_string(),
            subject_key: "key".to_string(),
            payload: json!({}),
            priority: 0,
        };

        let result = validate_request(&req);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("too long"));
    }

    #[test]
    fn test_validate_queue_name_invalid_chars() {
        let req = EnqueueRequest {
            queue: "invalid@queue!".to_string(),
            job_type: "test".to_string(),
            subject_key: "key".to_string(),
            payload: json!({}),
            priority: 0,
        };

        let result = validate_request(&req);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("alphanumeric"));
    }

    #[test]
    fn test_validate_priority_out_of_range() {
        let req = EnqueueRequest {
            queue: "test".to_string(),
            job_type: "test".to_string(),
            subject_key: "key".to_string(),
            payload: json!({}),
            priority: 101, // Out of range
        };

        let result = validate_request(&req);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("out of range"));
    }

    #[test]
    fn test_validate_payload_depth() {
        // Create deeply nested JSON
        let mut deep = json!({"level": 0});
        for i in 1..=35 {
            deep = json!({"level": i, "nested": deep});
        }

        let req = EnqueueRequest {
            queue: "test".to_string(),
            job_type: "test".to_string(),
            subject_key: "key".to_string(),
            payload: deep,
            priority: 0,
        };

        let result = validate_request(&req);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("deeply nested"));
    }

    #[test]
    fn test_validate_valid_request() {
        let req = EnqueueRequest {
            queue: "test_queue".to_string(),
            job_type: "test_job".to_string(),
            subject_key: "test_key".to_string(),
            payload: json!({"data": "value"}),
            priority: 50,
        };

        let result = validate_request(&req);
        assert!(result.is_ok());
    }
}
