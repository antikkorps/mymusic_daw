#[cfg(test)]
mod tests {
    use crate::sampler::loader::*;
    use std::path::PathBuf;

    #[test]
    fn test_load_mp3_support() {
        // Test that MP3 format is recognized
        let mp3_path = PathBuf::from("test.mp3");
        
        // This should not panic - just checking format recognition
        match load_sample(&mp3_path) {
            Err(msg) => {
                // Expected since file doesn't exist, but should be format-related error
                assert!(!msg.contains("Unsupported file format"));
            }
            Ok(_) => {
                // Unexpected success, but not an error
            }
        }
    }

    #[test]
    fn test_supported_formats() {
        let extensions = vec!["wav", "flac", "mp3"];
        
        for ext in extensions {
            let path = PathBuf::from(format!("test.{}", ext));
            match load_sample(&path) {
                Err(msg) => {
                    // Should not be "unsupported format" error
                    assert!(!msg.contains("Unsupported file format"), 
                           "Format {} should be supported but got: {}", ext, msg);
                }
                Ok(_) => {
                    // File doesn't exist, but format is supported
                }
            }
        }
    }

    #[test]
    fn test_unsupported_format() {
        let path = PathBuf::from("test.xyz");
        match load_sample(&path) {
            Err(msg) => {
                assert!(msg.contains("Unsupported file format"));
            }
            Ok(_) => {
                panic!("Unsupported format should not load successfully");
            }
        }
    }
}