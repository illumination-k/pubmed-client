use std::fs;
use std::path::Path;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_all_pmc_xml_files() {
        let xml_dir = Path::new("tests/test_data/pmc_xml");

        // Read all XML files in the directory
        let entries = fs::read_dir(xml_dir).expect("Failed to read test data directory");

        for entry in entries {
            let entry = entry.expect("Failed to read directory entry");
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("xml") {
                let xml_content = fs::read_to_string(&path)
                    .expect(&format!("Failed to read XML file: {:?}", path));

                println!("Testing file: {:?}", path.file_name().unwrap());

                // TODO: Add actual parsing test here once PmcClient parsing is implemented
                // For now, just verify the XML is not empty and contains PMC ID
                assert!(!xml_content.is_empty());
                assert!(xml_content.contains("<article"));
                assert!(xml_content.contains("PMC"));
            }
        }
    }
}
