# Feature Gap Analysis: pubmed-client vs Other PubMed Client Libraries

**Date**: 2026-02-20
**Compared with**: Bio.Entrez (Biopython), metapub, rentrez (R), entrezpy, easyPubMed (R), NCBI EDirect, PyMed, pubmedR, reutils (R), pubmed crate (Rust)

---

## Executive Summary

`pubmed-client` は PubMed/PMC クライアントとして多くの機能を持ち、特に PMC フルテキスト取得・Markdown 変換・図表抽出・複数言語バインディング（Python/Node.js/WASM）は他のライブラリにない強みである。しかし、NCBI E-utility の一部エンドポイント未対応や、`PubMedArticle` モデルの書誌情報フィールド不足など、既存ライブラリと比較して改善の余地がある。

---

## 1. E-utility エンドポイント対応状況

| E-utility    | 説明             | Bio.Entrez | rentrez | entrezpy | EDirect | **pubmed-client** |
| ------------ | ---------------- | ---------- | ------- | -------- | ------- | ----------------- |
| ESearch      | 検索             | YES        | YES     | YES      | YES     | **YES**           |
| EFetch       | レコード取得     | YES        | YES     | YES      | YES     | **YES**           |
| ELink        | 関連リンク       | YES        | YES     | YES      | YES     | **YES**           |
| EInfo        | DB情報           | YES        | YES     | NO       | YES     | **YES**           |
| ECitMatch    | 引用マッチ       | YES        | YES     | NO       | YES     | **YES**           |
| EGQuery      | 全DB検索         | YES        | YES     | NO       | YES     | **YES**           |
| **ESpell**   | スペル修正       | YES        | YES     | NO       | YES     | **NO**            |
| **ESummary** | 軽量サマリー     | YES        | YES     | YES      | YES     | **NO**            |
| **EPost**    | UID アップロード | YES        | YES     | YES      | YES     | **NO**            |

### 1.1 ESpell (スペル修正候補)

検索クエリのスペルミスを自動修正する機能。

- エンドポイント: `espell.fcgi?db=pubmed&term=<query>`
- 例: `"asthmaa OR alergies"` → `"asthma OR allergies"`
- ユースケース: 検索UI でのオートコレクト、クエリ品質の改善
- 実装工数: 小（単純なHTTPリクエスト + XMLパース）

### 1.2 ESummary (ドキュメントサマリー)

EFetch より軽量にメタデータを取得する手段。

- エンドポイント: `esummary.fcgi?db=pubmed&id=<uids>`
- EFetch との違い: フル XML ではなく DocSum (タイトル・著者・日付等の基本情報) を返す
- ユースケース: 大量の論文リスト表示、メタデータだけ必要な場面で EFetch より高速
- Version 2.0 XML スキーマではより詳細な情報も取得可能
- 実装工数: 中（レスポンスパーサーの新規作成が必要）

### 1.3 EPost (UID リストアップロード)

任意の PMID リストを NCBI History サーバーにアップロードし、WebEnv/query_key を取得する機能。

- エンドポイント: `epost.fcgi?db=pubmed&id=<uid_list>`
- 現状: `search_with_history()` でのみ WebEnv を取得可能。ユーザー独自の PMID リストをアップロードする手段がない
- ユースケース:
  - 外部から取得した PMID リストの一括処理
  - 複数の ESearch 結果を History サーバー上で AND/OR 結合
  - 大規模 UID リスト (10,000+) の効率的な処理
- 実装工数: 小〜中

---

## 2. PubMedArticle モデルの欠落フィールド

PubMed XML (`MedlineCitation`) に存在するが、現在の `PubMedArticle` 構造体に含まれていないフィールド:

### 2.1 基本書誌情報 (重要度: 高)

| フィールド             | XML パス                      | 説明                                   |
| ---------------------- | ----------------------------- | -------------------------------------- |
| `volume`               | `Journal/JournalIssue/Volume` | 巻号                                   |
| `issue`                | `Journal/JournalIssue/Issue`  | 号                                     |
| `pages`                | `Pagination/MedlinePgn`       | ページ範囲 (例: "123-130")             |
| `language`             | `Language`                    | 論文の言語 (例: "eng", "jpn")          |
| `journal_abbreviation` | `Journal/ISOAbbreviation`     | ISO ジャーナル略称 (例: "J Biol Chem") |
| `issn`                 | `Journal/ISSN`                | ISSN (print/electronic)                |

**影響**: これらは引用文字列の生成に必須。例えば NLM 形式の引用は `Author. Title. Journal. Year;Volume(Issue):Pages.` の形式。これらがないと正しい引用が生成できない。

**備考**: XML パーサーで `JournalIssue` はすでにデシリアライズされているが (`xml_types.rs:104`)、`PubDate` のみ抽出しており Volume/Issue は無視されている。`Journal` 構造体にも `ISOAbbreviation` や `ISSN` のフィールドがない。

### 2.2 出版履歴・ステータス (重要度: 中)

| フィールド           | XML パス                                                  | 説明                                        |
| -------------------- | --------------------------------------------------------- | ------------------------------------------- |
| `publication_status` | `PubmedData/PublicationStatus`                            | 出版状態 (epublish, ppublish, aheadofprint) |
| `received_date`      | `PubmedData/History/PubMedPubDate[@PubStatus='received']` | 投稿受領日                                  |
| `accepted_date`      | `PubmedData/History/PubMedPubDate[@PubStatus='accepted']` | 受理日                                      |
| `epub_date`          | `PubmedData/History/PubMedPubDate[@PubStatus='epublish']` | 電子出版日                                  |

### 2.3 助成金・データバンク情報 (重要度: 中)

| フィールド      | XML パス                               | 説明                                                        |
| --------------- | -------------------------------------- | ----------------------------------------------------------- |
| `grant_list`    | `MedlineCitation/Article/GrantList`    | 助成金情報 (機関名, 助成番号, 国)                           |
| `databank_list` | `MedlineCitation/Article/DataBankList` | データバンクアクセッション (GenBank, ClinicalTrials.gov 等) |

**備考**: PMC フルテキスト (`PmcFullText`) では `funding` フィールドが存在するが、PubMed メタデータ側の `GrantList` は未パース。

### 2.4 訂正・コメント情報 (重要度: 低〜中)

| フィールド             | XML パス                                  | 説明                                    |
| ---------------------- | ----------------------------------------- | --------------------------------------- |
| `comments_corrections` | `MedlineCitation/CommentsCorrectionsList` | 訂正、撤回、コメント、errata へのリンク |

---

## 3. 構造化アブストラクト

**現状**: XML の `<AbstractText Label="BACKGROUND">...</AbstractText>` を正しくパースしているが、全セクションを単一文字列に結合して `abstract_text: Option<String>` に格納している。

**他のライブラリ**: easyPubMed の `epm_parse()` や metapub ではセクション単位でアクセス可能。

**改善案**:

```rust
pub struct PubMedArticle {
    // 既存: 結合版 (後方互換性維持)
    pub abstract_text: Option<String>,
    // 新規: セクション構造を保持
    pub abstract_sections: Option<Vec<AbstractSection>>,
}

pub struct AbstractSection {
    pub label: Option<String>,       // "BACKGROUND", "METHODS", etc.
    pub nlm_category: Option<String>, // 正規化されたカテゴリ
    pub text: String,
}
```

---

## 4. 検索機能の差分

### 4.1 ソート (重要度: 中)

PubMed ESearch API は `sort` パラメータをサポート:

| sort 値       | 説明                  |
| ------------- | --------------------- |
| `relevance`   | 関連性順 (デフォルト) |
| `pub_date`    | 出版日順              |
| `Author`      | 著者名順              |
| `JournalName` | ジャーナル名順        |
| `most_recent` | 最新追加順            |

現在の `SearchQuery` ビルダーにはソート指定の手段がない。

### 4.2 クエリ翻訳 (重要度: 中)

ESearch レスポンスの `querytranslation` フィールドには PubMed がクエリをどう解釈したかの情報が含まれる:

- 例: `"asthma"` → `"asthma"[MeSH Terms] OR "asthma"[All Fields]`
- デバッグや検索精度改善に有用
- 現在の `ESearchData` レスポンス構造体にフィールドが存在しない

### 4.3 rettype/retmode の柔軟性 (重要度: 低)

現在 EFetch は常に `retmode=xml&rettype=abstract` でリクエスト。MEDLINE 形式 (`rettype=medline`) やテキスト形式 (`retmode=text`) の取得は不可。

---

## 5. 外部サービス連携

### 5.1 CrossRef 連携 (重要度: 中)

**metapub** が提供する機能:

| 機能                | 説明                                                          |
| ------------------- | ------------------------------------------------------------- |
| `pmid2doi`          | PMID から DOI を解決                                          |
| `doi2pmid`          | DOI から PMID を逆引き                                        |
| `CrossRefFetcher`   | DOI からメタデータ取得、メタデータから DOI 解決               |
| `PubMedArticle2doi` | PubMedArticle オブジェクトから CrossRef 経由で DOI を見つける |

当プロジェクトでは DOI は EFetch の XML から取得可能だが、DOI が未登録の場合に CrossRef で補完する機能がない。

### 5.2 PDF/フルテキスト URL 探索 (重要度: 低)

**metapub の FindIt**: 68 以上の出版社サイトから論文 PDF の直接 URL を特定 (97.1% カバー率)。エンバーゴ検出、合法アクセス検証も含む。

---

## 6. 出力フォーマット・引用生成

### 6.1 引用フォーマット生成 (重要度: 中)

| 形式      | 例                                                                             |
| --------- | ------------------------------------------------------------------------------ |
| NLM       | `Author AB. Title. Journal. Year;Vol(Issue):Pages. doi:...`                    |
| APA 7th   | `Author, A. B. (Year). Title. Journal, Vol(Issue), Pages. https://doi.org/...` |
| AMA       | `Author AB. Title. Journal. Year;Vol(Issue):Pages. doi:...`                    |
| Vancouver | `Author AB. Title. Journal. Year;Vol(Issue):Pages.`                            |

**前提条件**: volume, issue, pages フィールドが `PubMedArticle` モデルに必要 (セクション2.1参照)

### 6.2 文献管理ソフト向けエクスポート (重要度: 低〜中)

| 形式   | 対応ソフト                |
| ------ | ------------------------- |
| NBIB   | PubMed 標準形式           |
| RIS    | EndNote, Mendeley, Zotero |
| BibTeX | LaTeX                     |

---

## 7. その他の差分

| 機能                 | 説明                                                     | 対応ライブラリ                | 重要度                   |
| -------------------- | -------------------------------------------------------- | ----------------------------- | ------------------------ |
| マルチDB サポート    | PubMed/PMC 以外の NCBI DB (Nucleotide, Protein, Gene 等) | Bio.Entrez, rentrez, entrezpy | 低                       |
| パイプラインシステム | ESearch→EPost→EFetch を連鎖させるワークフロー            | entrezpy (Conduit)            | 低                       |
| 自動クエリ分割       | 10,000 件超の結果を自動分割取得                          | easyPubMed                    | 低 (search_all で代替可) |
| マルチスレッド取得   | 並列ダウンロード                                         | entrezpy                      | 低 (async で代替可)      |
| Retraction 検出      | 撤回論文フラグ                                           | PMC OA API (部分対応済)       | 中                       |

---

## 8. 当プロジェクトの優位性 (他にない機能)

以下は他のPubMedクライアントにはない、当プロジェクト独自の強み:

| 機能                     | 説明                                                                                 |
| ------------------------ | ------------------------------------------------------------------------------------ |
| PMC Markdown 変換        | フルテキストを設定可能な Markdown に変換 (TOC, YAML frontmatter, 参考文献スタイル等) |
| 図表抽出 (tar.gz)        | PMC OA API から tar.gz をダウンロードし、図表とキャプションを自動マッチング          |
| MCP サーバー             | AI アシスタント (Claude Desktop等) からの直接利用                                    |
| マルチ言語バインディング | Rust + Python (PyO3) + Node.js (NAPI) + WASM の 4 プラットフォーム                   |
| MeSH 類似度計算          | Jaccard 類似度による論文間 MeSH 比較                                                 |
| 国際共同研究検出         | 著者所属国の解析による自動判定                                                       |
| レスポンスキャッシュ     | moka ベースの in-memory キャッシュ (TTL, 容量設定可)                                 |
| 非同期ストリーミング     | 大規模結果セットの async Stream 取得                                                 |

---

## 9. 優先度別改善ロードマップ

### Phase 1: 基本書誌情報の充実 (高優先度)

1. `PubMedArticle` に volume, issue, pages, language, journal_abbreviation, issn を追加
2. `JournalIssue` XML パーサーの拡張 (Volume, Issue の抽出)
3. `Pagination` XML パーサーの追加

### Phase 2: 未対応 E-utility エンドポイント (高優先度)

4. ESpell 実装
5. ESummary 実装
6. EPost 実装

### Phase 3: 検索・モデル改善 (中優先度)

7. SearchQuery にソートオプション追加
8. SearchResult にクエリ翻訳を追加
9. 構造化アブストラクトのセクション保持
10. grant_list, publication_status, history dates のパース

### Phase 4: 出力・連携機能 (中優先度)

11. 引用フォーマット生成 (NLM, APA, AMA, Vancouver)
12. 文献管理エクスポート (RIS, BibTeX, NBIB)
13. CrossRef 連携 (DOI 解決)

### Phase 5: 拡張機能 (低優先度)

14. comments_corrections パース
15. databank_list パース
16. マルチ DB サポート

---

## 10. 全ライブラリ横断比較表

| 機能                    | Bio.Entrez | PyMed  | metapub | rentrez | easyPubMed | entrezpy | EDirect | pubmedR      | reutils   | **pubmed-client**      |
| ----------------------- | ---------- | ------ | ------- | ------- | ---------- | -------- | ------- | ------------ | --------- | ---------------------- |
| **言語**                | Python     | Python | Python  | R       | R          | Python   | CLI     | R            | R         | **Rust (+Py/JS/WASM)** |
| **E-utility 対応数**    | 9/9        | 3/9    | 4/9     | 8/9     | 2-3/9      | 5/9      | 9/9+    | 2/9          | 9/9       | **6/9**                |
| ESearch                 | YES        | YES    | YES     | YES     | YES        | YES      | YES     | YES          | YES       | **YES**                |
| EFetch                  | YES        | YES    | YES     | YES     | YES        | YES      | YES     | YES          | YES       | **YES**                |
| ELink                   | YES        | NO     | YES     | YES     | NO         | YES      | YES     | NO           | YES       | **YES**                |
| EInfo                   | YES        | NO     | NO      | YES     | NO         | NO       | YES     | NO           | YES       | **YES**                |
| ECitMatch               | YES        | NO     | YES     | YES     | NO         | NO       | YES     | NO           | YES       | **YES**                |
| EGQuery                 | YES        | NO     | NO      | YES     | NO         | NO       | YES     | NO           | YES       | **YES**                |
| **ESpell**              | YES        | NO     | NO      | NO      | NO         | NO       | YES     | NO           | YES       | **NO**                 |
| **ESummary**            | YES        | YES    | NO      | YES     | NO         | YES      | YES     | NO           | YES       | **NO**                 |
| **EPost**               | YES        | NO     | NO      | YES     | Partial    | YES      | YES     | NO           | YES       | **NO**                 |
| クエリビルダー          | NO         | NO     | NO      | NO      | Partial    | NO       | NO      | YES          | NO        | **YES**                |
| 高レベル Article 型     | NO         | YES    | YES     | Partial | YES        | NO       | NO      | YES          | NO        | **YES**                |
| WebEnv / History        | YES        | NO     | NO      | YES     | Internal   | YES      | YES     | NO           | YES       | **YES**                |
| 自動バッチ処理          | NO         | YES    | NO      | NO      | YES        | YES      | YES     | NO           | NO        | **YES**                |
| レート制限              | YES        | YES    | YES     | YES     | YES        | YES      | YES     | N/A          | YES       | **YES**                |
| API キー対応            | YES        | NO     | YES     | YES     | YES        | YES      | YES     | Optional     | YES       | **YES**                |
| キャッシュ              | NO         | NO     | YES     | NO      | NO         | YES      | NO      | NO           | NO        | **YES**                |
| PMC フルテキスト解析    | NO         | NO     | NO      | NO      | NO         | NO       | NO      | NO           | NO        | **YES**                |
| Markdown 変換           | NO         | NO     | NO      | NO      | NO         | NO       | NO      | NO           | NO        | **YES**                |
| 図表抽出                | NO         | NO     | NO      | NO      | NO         | NO       | NO      | NO           | NO        | **YES**                |
| MeSH パース             | XML 経由   | NO     | YES     | NO      | YES        | NO       | xtract  | NO           | NO        | **YES**                |
| 引用分析                | ELink      | NO     | YES     | ELink   | Reference  | NO       | OCC     | bibliometrix | ECitMatch | **ELink+ECitMatch**    |
| PDF URL 探索            | NO         | NO     | YES     | NO      | NO         | NO       | NO      | NO           | NO        | **NO**                 |
| CrossRef 連携           | NO         | NO     | YES     | NO      | NO         | NO       | NO      | NO           | NO        | **NO**                 |
| 引用フォーマット生成    | NO         | NO     | YES     | NO      | NO         | NO       | NO      | NO           | NO        | **NO**                 |
| RIS/BibTeX エクスポート | NO         | NO     | BibTeX  | NO      | NO         | NO       | NO      | NO           | NO        | **NO**                 |
| リトライ                | YES        | YES    | YES     | NO      | NO         | YES      | YES     | NO           | NO        | **YES**                |
| パイプライン            | Manual     | NO     | NO      | Manual  | NO         | Conduit  | Pipe    | NO           | Manual    | **Manual**             |
| MCP サーバー            | NO         | NO     | NO      | NO      | NO         | NO       | NO      | NO           | NO        | **YES**                |
| WASM 対応               | NO         | NO     | NO      | NO      | NO         | NO       | NO      | NO           | NO        | **YES**                |

### 主要な所見

1. **全 E-utility 対応**: NCBI EDirect (9/9+), Bio.Entrez (9/9), reutils (9/9) のみが全エンドポイントを網羅。pubmed-client は 6/9。
2. **PMC フルテキスト解析**: **どのライブラリも PMC フルテキストの構造的パース、Markdown 変換、図表抽出を提供していない**。これは pubmed-client の最大の差別化要因。
3. **クエリビルダー**: 型安全なビルダーパターンを持つのは pubmed-client と pubmedR のみ。
4. **Rust エコシステム**: crates.io の `pubmed` クレートは最低限の機能のみ (ドキュメント 0%)。pubmed-client はRust での PubMed クライアントとして最も包括的。
5. **JS/TS エコシステム**: スタンドアロンの PubMed クライアントは事実上不在。2025-2026 年のトレンドは MCP サーバー化。pubmed-client-napi/wasm はこの空白を埋める唯一の選択肢。

---

## References

- [Bio.Entrez Documentation (Biopython)](https://biopython.org/docs/latest/api/Bio.Entrez.html)
- [metapub (GitHub)](https://github.com/metapub/metapub)
- [rentrez (CRAN)](https://cran.r-project.org/web/packages/rentrez/rentrez.pdf)
- [entrezpy Documentation](https://entrezpy.readthedocs.io/)
- [easyPubMed (GitHub)](https://github.com/dami82/easyPubMed)
- [NCBI EDirect Documentation](https://www.ncbi.nlm.nih.gov/books/NBK179288/)
- [NCBI E-utilities In-Depth](https://www.ncbi.nlm.nih.gov/books/NBK25499/)
- [NLM E-utilities Guide](https://www.nlm.nih.gov/dataguide/eutilities/utilities.html)
- [reutils (GitHub)](https://github.com/gschofl/reutils)
- [pubmedR (CRAN)](https://cran.r-project.org/web/packages/pubmedR/pubmedR.pdf)
- [bibliometrix](https://github.com/massimoaria/bibliometrix)
- [pubmed crate (crates.io)](https://crates.io/crates/pubmed)
- [PyMed (GitHub)](https://github.com/gijswobben/pymed)
- [pubmed.mineR (CRAN)](https://cran.r-project.org/web/packages/pubmed.mineR/pubmed.mineR.pdf)
