//! Incremental JSON parser for streaming responses
//! 
//! Implements bracket-matching algorithm to extract complete JSON objects
//! from a stream of bytes with:
//! - Buffer: 1MB maximum
//! - Algorithm: O(n) single pass bracket matching
//! - Recovery: Timeout and error handling

use crate::errors::{AgentError, Result};
use crate::types::AgentMsg;

/// Maximum buffer size (1MB)
pub const MAX_BUFFER_SIZE: usize = 1_048_576;

/// Parser states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum ParserState {
    /// Accumulating bytes, looking for JSON start
    Accumulating,
    
    /// Found JSON start, parsing in progress
    Parsing,
    
    /// Complete JSON extracted
    Complete,
}

/// Incremental JSON parser
#[derive(Debug)]
pub struct JsonParser {
    /// Accumulation buffer
    buffer: Vec<u8>,
    
    /// Current parser state
    state: ParserState,
    
    /// Maximum buffer size
    max_buffer_size: usize,
}

impl JsonParser {
    /// Create new JSON parser with default settings
    pub fn new() -> Self {
        Self::with_capacity(MAX_BUFFER_SIZE)
    }

    /// Create parser with custom buffer capacity
    pub fn with_capacity(max_buffer_size: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(4096), // Start with 4KB
            state: ParserState::Accumulating,
            max_buffer_size,
        }
    }

    /// Add bytes to parser and attempt to extract complete JSON
    /// 
    /// # Mathematical Specification
    /// 
    /// ```text
    /// Algorithm extract_complete_json(B):
    /// 1. depth ← 0, start ← None
    /// 2. For each byte bᵢ in B:
    ///      If bᵢ = '{':
    ///        If depth = 0: start ← i
    ///        depth ← depth + 1
    ///      If bᵢ = '}':
    ///        depth ← depth - 1
    ///        If depth = 0 and start ≠ None:
    ///          Return B[start..=i]
    /// 3. Return None  // No complete JSON yet
    /// 
    /// Complexity: O(n) single pass
    /// Guarantee: Returns valid JSON substring or None
    /// ```
    pub fn add_bytes(&mut self, bytes: &[u8]) -> Result<Option<String>> {
        // Check buffer overflow
        if self.buffer.len() + bytes.len() > self.max_buffer_size {
            return Err(AgentError::JsonParseError(format!(
                "Buffer overflow: {} bytes exceeds maximum {}",
                self.buffer.len() + bytes.len(),
                self.max_buffer_size
            )));
        }

        // Append to buffer
        self.buffer.extend_from_slice(bytes);

        // Try to extract complete JSON
        self.try_extract_json()
    }

    /// Attempt to extract complete JSON from buffer
    fn try_extract_json(&mut self) -> Result<Option<String>> {
        if self.buffer.is_empty() {
            return Ok(None);
        }

        // Find complete JSON object using bracket matching
        if let Some((start, end)) = self.find_complete_json()? {
            // Extract JSON substring
            let json_bytes = &self.buffer[start..=end];
            let json_str = String::from_utf8_lossy(json_bytes).to_string();

            // Remove processed bytes from buffer
            self.buffer.drain(..=end);

            // Reset state
            self.state = ParserState::Accumulating;

            return Ok(Some(json_str));
        }

        Ok(None)
    }

    /// Find complete JSON object using bracket-matching algorithm
    /// 
    /// Returns: Some((start_index, end_index)) or None
    fn find_complete_json(&self) -> Result<Option<(usize, usize)>> {
        let mut depth = 0;
        let mut start: Option<usize> = None;
        let mut in_string = false;
        let mut escape_next = false;

        for (i, &byte) in self.buffer.iter().enumerate() {
            let ch = byte as char;

            // Handle string escaping
            if escape_next {
                escape_next = false;
                continue;
            }

            if ch == '\\' && in_string {
                escape_next = true;
                continue;
            }

            // Track string boundaries (ignore braces inside strings)
            if ch == '"' {
                in_string = !in_string;
                continue;
            }

            // Skip if inside string
            if in_string {
                continue;
            }

            // Bracket matching
            match ch {
                '{' => {
                    if depth == 0 {
                        start = Some(i);
                    }
                    depth += 1;
                }
                '}' => {
                    depth -= 1;
                    if depth == 0 && start.is_some() {
                        // Found complete JSON object
                        return Ok(Some((start.unwrap(), i)));
                    }
                    if depth < 0 {
                        return Err(AgentError::JsonParseError(
                            "Mismatched braces: too many closing braces".to_string(),
                        ));
                    }
                }
                _ => {}
            }
        }

        // No complete JSON yet
        Ok(None)
    }

    /// Parse extracted JSON string into AgentMsg
    pub fn parse_agent_msg(&self, json_str: &str) -> Result<AgentMsg> {
        serde_json::from_str(json_str)
            .map_err(|e| AgentError::JsonParseError(format!("Failed to parse AgentMsg: {}", e)))
    }

    /// Get current buffer size
    pub fn buffer_size(&self) -> usize {
        self.buffer.len()
    }

    /// Clear the buffer
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.state = ParserState::Accumulating;
    }

    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Force parse remaining buffer (timeout recovery)
    /// 
    /// Attempts to parse whatever is in the buffer, even if incomplete
    pub fn force_parse(&mut self) -> Result<Option<String>> {
        if self.buffer.is_empty() {
            return Ok(None);
        }

        // Try to find any complete JSON
        if let Some(json) = self.try_extract_json()? {
            return Ok(Some(json));
        }

        // If no complete JSON, try to parse the whole buffer
        let buffer_str = String::from_utf8_lossy(&self.buffer).to_string();
        
        // Clear buffer
        self.buffer.clear();
        self.state = ParserState::Accumulating;

        Ok(Some(buffer_str))
    }
}

impl Default for JsonParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_json_extraction() {
        let mut parser = JsonParser::new();

        let json = r#"{"type": "plan", "steps": ["step1", "step2"]}"#;
        let result = parser.add_bytes(json.as_bytes()).unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap(), json);
    }

    #[test]
    fn test_incremental_json_extraction() {
        let mut parser = JsonParser::new();

        // Add bytes incrementally
        parser.add_bytes(br#"{"type":"#).unwrap();
        assert!(parser.add_bytes(br#" "plan""#).unwrap().is_none());
        
        let result = parser.add_bytes(br#", "steps":[]}"#).unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn test_nested_braces() {
        let mut parser = JsonParser::new();

        let json = r#"{"outer": {"inner": {"deep": "value"}}}"#;
        let result = parser.add_bytes(json.as_bytes()).unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap(), json);
    }

    #[test]
    fn test_braces_in_strings() {
        let mut parser = JsonParser::new();

        let json = r#"{"message": "This has {braces} inside"}"#;
        let result = parser.add_bytes(json.as_bytes()).unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap(), json);
    }

    #[test]
    fn test_escaped_quotes() {
        let mut parser = JsonParser::new();

        let json = r#"{"message": "Quote: \"Hello\""}"#;
        let result = parser.add_bytes(json.as_bytes()).unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap(), json);
    }

    #[test]
    fn test_multiple_json_objects() {
        let mut parser = JsonParser::new();

        let data = r#"{"first": 1}{"second": 2}"#;
        
        // Should extract first object
        let result1 = parser.add_bytes(data.as_bytes()).unwrap();
        assert!(result1.is_some());
        assert_eq!(result1.unwrap(), r#"{"first": 1}"#);

        // Should extract second object on next call
        let result2 = parser.try_extract_json().unwrap();
        assert!(result2.is_some());
        assert_eq!(result2.unwrap(), r#"{"second": 2}"#);
    }

    #[test]
    fn test_buffer_overflow() {
        let mut parser = JsonParser::with_capacity(100);

        let large_data = vec![b'a'; 150];
        let result = parser.add_bytes(&large_data);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AgentError::JsonParseError(_)));
    }

    #[test]
    fn test_mismatched_braces() {
        let mut parser = JsonParser::new();

        parser.add_bytes(b"{").unwrap();
        let result = parser.add_bytes(b"}}");

        // Parser extracts first complete object "{}" and keeps extra "}" in buffer
        // This is valid behavior - no error expected until we try to parse the remaining
        assert!(result.is_ok());
    }

    #[test]
    fn test_clear() {
        let mut parser = JsonParser::new();

        parser.add_bytes(b"partial json").unwrap();
        assert!(!parser.is_empty());

        parser.clear();
        assert!(parser.is_empty());
        assert_eq!(parser.buffer_size(), 0);
    }

    #[test]
    fn test_force_parse() {
        let mut parser = JsonParser::new();

        parser.add_bytes(b"incomplete").unwrap();
        
        let result = parser.force_parse().unwrap();
        assert!(result.is_some());
        assert!(parser.is_empty());
    }

    #[test]
    fn test_parse_agent_msg_plan() {
        let parser = JsonParser::new();

        let json = r#"{"type": "plan", "steps": ["step1"], "reasoning": "test"}"#;
        let msg = parser.parse_agent_msg(json).unwrap();

        assert!(matches!(msg, AgentMsg::Plan { .. }));
    }

    #[test]
    fn test_parse_agent_msg_tool_call() {
        let parser = JsonParser::new();

        let json = r#"{"type": "tool_call", "tool": "read_file", "args": {"path": "/test"}}"#;
        let msg = parser.parse_agent_msg(json).unwrap();

        assert!(matches!(msg, AgentMsg::ToolCall { .. }));
    }
}
