# Roadmap: pubmed-client の進化計画

本ドキュメントはプロジェクトの現状分析に基づく、今後の進化方向性を示すものです。

## 現状の強み

- **NCBI E-utilities の幅広いカバレッジ**: ESearch, EFetch, ELink, EInfo 実装済み
- **マルチランゲージ対応**: Rust コア + WASM / Python (PyO3) / Node.js (NAPI) バインディング
- **CLI ツール**: 検索、図抽出、Markdown 変換、メタデータ抽出、ID 変換
- **MCP サーバー**: AI アシスタント連携の基盤
- **堅牢な基盤**: レートリミッター、リトライ、キャッシュ、構造化ログ

---

## 提案一覧

### 1. MCP サーバーのツール拡充 [優先度: 最高]

**現状**: `search_pubmed` と `get_pmc_markdown` の 2 ツールのみ。
**提案**: コアライブラリに既に実装済みの機能を MCP に公開する。

- `fetch_article_metadata` — PMID 指定でメタデータ取得（abstract, authors, MeSH terms）
- `get_related_articles` — 関連論文の探索
- `get_citations` — 被引用論文の取得
- `check_oa_availability` — OA 状態チェック
- `convert_ids` — PMID ↔ PMCID ↔ DOI の相互変換
- `extract_figures` — 図のキャプション・メタデータ取得

**根拠**: 既存コードのラッパー作成のみで済むため、実装コスト対リターンが最も高い。

---

### 2. バッチ処理・並列処理の強化 [優先度: 高]

**現状**: `search_and_fetch` は逐次処理。`efetch` は本来複数 PMID を一括受付可能。

- **並列 fetch**: `FuturesUnordered` 等で並列フェッチ。レートリミッターは維持
- **バルクフェッチ API**: 1 リクエストで複数 PMID を取得（カンマ区切り）
- **進捗コールバック / チャンネル**: CLI・バインディング側で進捗表示を容易にする

---

### 3. Structured Abstract の対応 [優先度: 高]

**現状**: `abstract_text: Option<String>` に平坦化。PubMed XML には Background, Methods, Results, Conclusions などのセクション構造がある。

```rust
pub struct StructuredAbstract {
    pub sections: Vec<AbstractSection>,
    pub full_text: String, // 後方互換
}

pub struct AbstractSection {
    pub label: String,
    pub text: String,
}
```

**効果**: LLM への構造化入力として価値が上がる。研究メソドロジーのフィルタリングが可能に。

---

### 4. NCBI E-utilities API のカバレッジ拡大 [優先度: 高]

未実装の E-utilities:

- **ESpell** — クエリの自動スペル修正提案
- **ECitMatch** — DOI やジャーナル名+巻+ページから PMID を逆引き
- **EGQuery** — 全 NCBI データベースでのヒット数を一括取得
- **ESummary** — EFetch より軽量なサマリー取得

---

### 5. データエクスポート・変換機能 [優先度: 中]

研究者のワークフローに合わせた出力形式:

- **BibTeX / RIS エクスポート**: 文献管理ソフト（Zotero, Mendeley）へのインポート
- **CSV エクスポート**: CLI の `metadata` コマンドの拡張
- **LaTeX / reStructuredText 変換**: Markdown 以外の学術文書形式

---

### 6. async Python バインディング [優先度: 中]

**現状**: ブロッキング API（内部で Tokio ランタイム作成）。

```python
import asyncio
from pubmed_client import AsyncClient

async def main():
    client = AsyncClient()
    results = await asyncio.gather(
        client.pubmed.search_and_fetch("covid-19 vaccine", 5),
        client.pubmed.search_and_fetch("covid-19 treatment", 5),
    )
```

`pyo3-asyncio` crate で実現可能。データパイプラインでの活用が広がる。

---

### 7. ローカルデータベース / 永続キャッシュ [優先度: 中]

**現状**: moka によるインメモリキャッシュのみ。

- **SQLite 永続キャッシュ**: フェッチ済み論文メタデータの永続化
- **全文検索インデックス**: tantivy でローカル論文の高速検索
- **オフライン対応**: 一度取得した論文はネットワーク不要に

---

### 8. 引用ネットワーク分析 [優先度: 中]

`get_related_articles`, `get_citations` の基盤を活用:

- **引用グラフ構築**: N ホップの引用ネットワーク
- **重要論文の特定**: PageRank 的アルゴリズム
- **時系列分析**: 分野のトレンド可視化

---

### 9. テスト基盤の強化 [優先度: 中]

- **プロパティベーステスト**: `proptest` で SearchQuery ビルダーのエッジケース自動生成
- **スナップショットテスト**: `insta` でパーサー結果の回帰検出
- **wiremock 活用拡大**: 確定的なテストスイートの構築

---

### 10. ドキュメント・エコシステム [優先度: 低]

- Jupyter Notebook 連携例（Python バインディングの論文分析チュートリアル）
- examples/ ディレクトリの拡充（ユースケースごとの実行可能な例）
- `git-cliff` による changelog 自動生成

---

## 優先度マトリクス

| 提案                                    | ユーザー価値   | 実装コスト | 推奨優先度 |
| --------------------------------------- | -------------- | ---------- | ---------- |
| MCP ツール拡充                          | 高             | 低         | **最優先** |
| バッチ並列処理                          | 高             | 中         | 高         |
| Structured Abstract                     | 中             | 低         | 高         |
| E-utilities 拡張 (ESpell, ECitMatch 等) | 中             | 低         | 高         |
| BibTeX/RIS エクスポート                 | 中             | 低         | 中         |
| async Python                            | 中             | 中         | 中         |
| SQLite 永続キャッシュ                   | 中             | 中         | 中         |
| 引用ネットワーク分析                    | 高             | 高         | 中         |
| テスト基盤強化                          | 低(開発者向け) | 低         | 中         |
| 全文検索インデックス                    | 高             | 高         | 低         |
