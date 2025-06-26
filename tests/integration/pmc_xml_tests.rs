mod common;
use common::get_pmc_xml_test_cases;
use tracing::{info, warn};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_all_pmc_xml_files() {
        let test_cases = get_pmc_xml_test_cases();

        if test_cases.is_empty() {
            warn!("No XML test files found in tests/test_data/pmc_xml");
            return;
        }

        for test_case in test_cases {
            info!(filename = test_case.filename(), "Testing file");

            let xml_content = test_case.read_xml_content_or_panic();

            // Basic validation
            assert!(!xml_content.is_empty(), "XML file should not be empty");
            assert!(
                xml_content.contains("<article"),
                "Should contain article tag"
            );
            assert!(xml_content.contains("PMC"), "Should contain PMC reference");

            info!(filename = test_case.filename(), "Basic validation passed");
        }
    }

    #[test]
    fn test_xml_test_case_functionality() {
        let test_cases = get_pmc_xml_test_cases();

        if let Some(first_case) = test_cases.first() {
            // Test filename extraction
            assert!(first_case.filename().ends_with(".xml"));
            assert!(!first_case.pmcid.is_empty());

            // Test content reading
            let content = first_case.read_xml_content();
            assert!(content.is_ok());

            // Test panic-free content reading
            let content_panic = first_case.read_xml_content_or_panic();
            assert!(!content_panic.is_empty());

            info!("PmcXmlTestCase functionality validated");
        }
    }

    #[test]
    fn test_specific_xml_file_access() {
        use common::get_pmc_xml_test_case;

        let test_cases = get_pmc_xml_test_cases();

        if let Some(first_case) = test_cases.first() {
            let filename = first_case.filename();
            let specific_case = get_pmc_xml_test_case(filename);

            assert!(specific_case.is_some());
            let specific_case = specific_case.unwrap();
            assert_eq!(specific_case.filename(), filename);
            assert_eq!(specific_case.pmcid, first_case.pmcid);

            info!("Specific XML file access validated");
        }
    }

    #[test]
    fn test_nonexistent_file_handling() {
        use common::get_pmc_xml_test_case;

        let nonexistent = get_pmc_xml_test_case("nonexistent_file.xml");
        assert!(nonexistent.is_none());

        info!("Nonexistent file handling validated");
    }
}
