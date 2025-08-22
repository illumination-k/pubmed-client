use pubmed_client_rs::pmc::parser::PmcXmlParser;
use std::fs;
use tracing::info;

#[test]
fn test_supplementary_material_extraction_from_real_xml() {
    let test_data_dir = "tests/integration/test_data/pmc_xml";

    // Read PMC XML files that contain supplementary materials
    let test_files = vec![
        "PMC10821037.xml", // This file contains supplementary materials based on previous analysis
    ];

    for file_name in test_files {
        let file_path = format!("{}/{}", test_data_dir, file_name);

        if let Ok(xml_content) = fs::read_to_string(&file_path) {
            let pmcid = file_name.replace(".xml", "");
            let result = PmcXmlParser::parse(&xml_content, &pmcid);

            assert!(result.is_ok(), "Failed to parse XML from {}", file_name);

            let article = result.unwrap();
            info!(file_name = %file_name, "Testing file");
            info!(
                supplementary_materials_count = article.supplementary_materials.len(),
                "Found supplementary materials"
            );

            // Check if supplementary materials were found
            for (i, material) in article.supplementary_materials.iter().enumerate() {
                info!(
                    material_index = i + 1,
                    material_id = %material.id,
                    content_type = ?material.content_type,
                    title = ?material.title,
                    file_url = ?material.file_url,
                    is_tar_file = material.is_tar_file(),
                    is_archive = material.is_archive(),
                    file_extension = ?material.get_file_extension(),
                    "Supplementary material details"
                );

                // Basic validation
                assert!(
                    material.file_url.is_some(),
                    "Supplementary material should have a file URL"
                );
                assert!(
                    !material.id.is_empty(),
                    "Supplementary material should have an ID"
                );
            }

            // Test the article's supplementary material helper methods
            let tar_files = article.get_tar_files();
            let archive_files = article.get_archive_files();

            info!(
                tar_files_count = tar_files.len(),
                archive_files_count = archive_files.len(),
                "Archive files summary"
            );

            info!(
                has_supplementary_materials = article.has_supplementary_materials(),
                "Article supplementary materials status"
            );
        } else {
            info!(file_path = %file_path, "Warning: Could not read test file");
        }
    }
}

#[test]
fn test_supplementary_material_parsing_with_mock_data() {
    let mock_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
    <article>
        <front>
            <article-meta>
                <article-id pub-id-type="pmc">PMC1234567</article-id>
                <title-group>
                    <article-title>Test Article with Supplementary Materials</article-title>
                </title-group>
            </article-meta>
        </front>
        <body>
            <sec>
                <title>Methods</title>
                <p>This is the methods section.</p>
            </sec>
        </body>
        <back>
            <app-group>
                <supplementary-material id="supp-data-1" content-type="local-data" position="float">
                    <caption>
                        <title>Supplementary Data Archive</title>
                        <p>Complete dataset archive containing all raw data files.</p>
                    </caption>
                    <media xlink:href="supplementary-data.tar.gz"/>
                </supplementary-material>
                <supplementary-material id="supp-figures" content-type="local-data">
                    <caption>
                        <title>Supplementary Figures</title>
                        <p>Additional figures and charts.</p>
                    </caption>
                    <media xlink:href="supplementary-figures.zip"/>
                </supplementary-material>
                <supplementary-material id="supp-code" content-type="local-data">
                    <caption>
                        <title>Source Code</title>
                        <p>Analysis source code and scripts.</p>
                    </caption>
                    <media xlink:href="analysis-code.tar.bz2"/>
                </supplementary-material>
            </app-group>
        </back>
    </article>"#;

    let result = PmcXmlParser::parse(mock_xml, "PMC1234567");
    assert!(result.is_ok());

    let article = result.unwrap();

    // Should have 3 supplementary materials
    assert_eq!(article.supplementary_materials.len(), 3);

    // Check the first material (tar.gz)
    let tar_material = &article.supplementary_materials[0];
    assert_eq!(tar_material.id, "supp-data-1");
    assert_eq!(tar_material.content_type, Some("local-data".to_string()));
    assert_eq!(tar_material.position, Some("float".to_string()));
    assert_eq!(
        tar_material.title,
        Some("Supplementary Data Archive".to_string())
    );
    assert_eq!(
        tar_material.file_url,
        Some("supplementary-data.tar.gz".to_string())
    );
    assert!(tar_material.is_tar_file());
    assert!(tar_material.is_archive());

    // Check the second material (zip)
    let zip_material = &article.supplementary_materials[1];
    assert_eq!(zip_material.id, "supp-figures");
    assert_eq!(
        zip_material.file_url,
        Some("supplementary-figures.zip".to_string())
    );
    assert!(!zip_material.is_tar_file());
    assert!(zip_material.is_archive());

    // Check the third material (tar.bz2)
    let bz2_material = &article.supplementary_materials[2];
    assert_eq!(bz2_material.id, "supp-code");
    assert_eq!(
        bz2_material.file_url,
        Some("analysis-code.tar.bz2".to_string())
    );
    assert!(bz2_material.is_tar_file());
    assert!(bz2_material.is_archive());

    // Test helper methods
    assert!(article.has_supplementary_materials());

    let tar_files = article.get_tar_files();
    assert_eq!(tar_files.len(), 2); // tar.gz and tar.bz2

    let archive_files = article.get_archive_files();
    assert_eq!(archive_files.len(), 3); // All three are archives

    let local_data_materials = article.get_supplementary_materials_by_type("local-data");
    assert_eq!(local_data_materials.len(), 3); // All have local-data type
}

#[test]
fn test_tar_file_variants() {
    let test_cases = vec![
        ("dataset.tar", true),
        ("dataset.tar.gz", true),
        ("dataset.tar.bz2", true),
        ("dataset.tgz", true),
        ("dataset.zip", false),
        ("dataset.rar", false),
        ("dataset.7z", false),
        ("document.pdf", false),
    ];

    for (filename, expected_is_tar) in test_cases {
        let mock_xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
        <article>
            <supplementary-material id="test-supp" content-type="local-data">
                <caption>
                    <title>Test File</title>
                </caption>
                <media xlink:href="{}"/>
            </supplementary-material>
        </article>"#,
            filename
        );

        let result = PmcXmlParser::parse(&mock_xml, "PMC1234567");
        assert!(result.is_ok());

        let article = result.unwrap();
        assert_eq!(article.supplementary_materials.len(), 1);

        let material = &article.supplementary_materials[0];
        assert_eq!(
            material.is_tar_file(),
            expected_is_tar,
            "File {} should{}be detected as tar file",
            filename,
            if expected_is_tar { " " } else { " not " }
        );
    }
}
