use pubmed_client::pubmed::parser::parse_article_from_xml;

const TEST_XML_WITH_ABSTRACT: &str = r#"<?xml version="1.0" ?>
<!DOCTYPE PubmedArticleSet PUBLIC "-//NLM//DTD PubMedArticle, 1st January 2025//EN" "https://dtd.nlm.nih.gov/ncbi/pubmed/out/pubmed_250101.dtd">
<PubmedArticleSet>
<PubmedArticle>
<MedlineCitation Status="MEDLINE" Owner="NLM" IndexingMethod="Manual">
<PMID Version="1">31978945</PMID>
<Article PubModel="Print-Electronic">
<Journal>
<Title>The New England journal of medicine</Title>
</Journal>
<ArticleTitle>A Novel Coronavirus from Patients with Pneumonia in China, 2019.</ArticleTitle>
<Abstract>
<AbstractText>In December 2019, a cluster of patients with pneumonia of unknown cause was linked to a seafood wholesale market in Wuhan, China. A previously unknown betacoronavirus was discovered through the use of unbiased sequencing in samples from patients with pneumonia. Human airway epithelial cells were used to isolate a novel coronavirus, named 2019-nCoV, which formed a clade within the subgenus sarbecovirus, Orthocoronavirinae subfamily. Different from both MERS-CoV and SARS-CoV, 2019-nCoV is the seventh member of the family of coronaviruses that infect humans. Enhanced surveillance and further investigation are ongoing. (Funded by the National Key Research and Development Program of China and the National Major Project for Control and Prevention of Infectious Disease in China.).</AbstractText>
</Abstract>
<AuthorList CompleteYN="Y">
<Author ValidYN="Y">
<LastName>Zhu</LastName>
<ForeName>Na</ForeName>
</Author>
<Author ValidYN="Y">
<LastName>Zhang</LastName>
<ForeName>Dingyu</ForeName>
</Author>
</AuthorList>
<PublicationTypeList>
<PublicationType UI="D016428">Journal Article</PublicationType>
</PublicationTypeList>
</Article>
</MedlineCitation>
</PubmedArticle>
</PubmedArticleSet>"#;

const TEST_XML_WITHOUT_ABSTRACT: &str = r#"<?xml version="1.0" ?>
<!DOCTYPE PubmedArticleSet PUBLIC "-//NLM//DTD PubMedArticle, 1st January 2025//EN" "https://dtd.nlm.nih.gov/ncbi/pubmed/out/pubmed_250101.dtd">
<PubmedArticleSet>
<PubmedArticle>
<MedlineCitation Status="MEDLINE" Owner="NLM" IndexingMethod="Manual">
<PMID Version="1">33515491</PMID>
<Article PubModel="Print-Electronic">
<Journal>
<Title>Lancet (London, England)</Title>
</Journal>
<ArticleTitle>Resurgence of COVID-19 in Manaus, Brazil, despite high seroprevalence.</ArticleTitle>
<AuthorList CompleteYN="Y">
<Author ValidYN="Y">
<LastName>Sabino</LastName>
<ForeName>Ester C</ForeName>
</Author>
</AuthorList>
<PublicationTypeList>
<PublicationType UI="D016428">Journal Article</PublicationType>
</PublicationTypeList>
</Article>
</MedlineCitation>
</PubmedArticle>
</PubmedArticleSet>"#;

#[test]
fn test_parse_article_with_abstract() {
    let result = parse_article_from_xml(TEST_XML_WITH_ABSTRACT, "31978945");

    assert!(result.is_ok());
    let article = result.unwrap();

    assert_eq!(article.pmid, "31978945");
    assert_eq!(
        article.title,
        "A Novel Coronavirus from Patients with Pneumonia in China, 2019."
    );
    assert_eq!(article.journal, "The New England journal of medicine");
    assert_eq!(article.authors.len(), 2);
    assert_eq!(article.authors[0], "Na Zhu");
    assert_eq!(article.authors[1], "Dingyu Zhang");
    assert_eq!(article.article_types, vec!["Journal Article"]);

    // Most importantly, check that the abstract is parsed correctly
    assert!(article.abstract_text.is_some());
    let abstract_text = article.abstract_text.unwrap();
    assert!(abstract_text.contains("In December 2019"));
    assert!(abstract_text.contains("2019-nCoV"));
    assert!(abstract_text.contains("Enhanced surveillance and further investigation are ongoing"));
}

#[test]
fn test_parse_article_without_abstract() {
    let result = parse_article_from_xml(TEST_XML_WITHOUT_ABSTRACT, "33515491");

    assert!(result.is_ok());
    let article = result.unwrap();

    assert_eq!(article.pmid, "33515491");
    assert_eq!(
        article.title,
        "Resurgence of COVID-19 in Manaus, Brazil, despite high seroprevalence."
    );
    assert_eq!(article.journal, "Lancet (London, England)");
    assert_eq!(article.authors.len(), 1);
    assert_eq!(article.authors[0], "Ester C Sabino");

    // Abstract should be None for this article
    assert!(article.abstract_text.is_none());
}

#[test]
fn test_parse_invalid_xml() {
    let invalid_xml = "<invalid>xml</not_closed>";
    let result = parse_article_from_xml(invalid_xml, "12345");

    assert!(result.is_err());
}

#[test]
fn test_parse_empty_xml() {
    let empty_xml = r#"<?xml version="1.0" ?>
    <PubmedArticleSet>
    </PubmedArticleSet>"#;
    let result = parse_article_from_xml(empty_xml, "12345");

    assert!(result.is_err());
    match result {
        Err(pubmed_client::error::PubMedError::ArticleNotFound { pmid }) => {
            assert_eq!(pmid, "12345");
        }
        _ => panic!("Expected ArticleNotFound error"),
    }
}
