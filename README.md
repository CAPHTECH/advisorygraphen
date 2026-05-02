# AdvisoryGraphen 実装ドキュメント群

AdvisoryGraphen は、コンサルティング業務を HigherGraphen の高次構造へ写像し、根拠、仮説、制約、未解決障害、提案候補、レビュー状態、投影結果を Rust で扱えるようにするための実装可能なプロダクト仕様である。

このドキュメント群は、最初の実装を **Rust workspace + CLI + JSON schema + agent skill + projection** として立ち上げることを目的にしている。最初から hosted SaaS や複雑な UI を実装しない。まず `advisorygraphen` CLI とファイルベースの構造モデルで、技術顧問・アーキテクチャレビュー・プロダクト意思決定支援の MVP を成立させる。

## 目的

AdvisoryGraphen は次の問題を解く。

1. コンサルタントの主張、顧客資料、AI 推論、レビュー済み結論を混同しない。
2. 提案を即座に事実や承認済み施策に昇格させず、`CompletionCandidate` としてレビュー可能に保つ。
3. レポートをモデル本体にしない。レポート、開発者向けアクション、監査証跡、AI 操作用ビューは、すべて構造から生成される `Projection` とする。
4. 継続顧問で、過去の意思決定、根拠、未解決 obstruction、採否済み candidate を append-only に追跡する。
5. 顧客固有の解釈パッケージや商用ノウハウを、公開コアとは分離して蓄積する。

## 最初の実装単位

MVP は `technical_advisory` interpretation package に限定する。

対象入力:

- アーキテクチャ説明
- ADR
- API/DB/モジュール一覧
- GitHub Issue / PR 要約
- 顧客ヒアリングメモ
- 要求、制約、ロードマップ
- 既存テストや検証記録

対象出力:

- `executive` projection: 経営判断用要約
- `developer_action` projection: 実装担当者向けアクション
- `audit_trace` projection: 根拠、レビュー状態、情報損失
- `ai_agent` projection: AI エージェントが継続作業するための構造

## 推奨読み順

1. [`docs/00-product-charter.md`](docs/00-product-charter.md)
2. [`docs/01-domain-model.md`](docs/01-domain-model.md)
3. [`docs/02-rust-workspace.md`](docs/02-rust-workspace.md)
4. [`docs/03-data-contracts.md`](docs/03-data-contracts.md)
5. [`docs/05-reasoning-invariants.md`](docs/05-reasoning-invariants.md)
6. [`docs/06-completion-and-review-workflow.md`](docs/06-completion-and-review-workflow.md)
7. [`docs/07-projections.md`](docs/07-projections.md)
8. [`docs/08-cli-contract.md`](docs/08-cli-contract.md)
9. [`docs/12-implementation-roadmap.md`](docs/12-implementation-roadmap.md)
10. [`docs/13-testing-acceptance.md`](docs/13-testing-acceptance.md)

## リポジトリ構成案

```text
advisorygraphen/
  Cargo.toml
  crates/
    advisorygraphen-core/
    advisorygraphen-lift/
    advisorygraphen-interpretation/
    advisorygraphen-reasoning/
    advisorygraphen-projection/
    advisorygraphen-runtime/
  tools/
    advisorygraphen-cli/
  schemas/
    advisorygraphen/
  examples/
    technical-advisory/
  skills/
    advisorygraphen/
  docs/
  adrs/
```

## MVP の CLI イメージ

```sh
advisorygraphen lift \
  --input examples/technical-advisory/direct-db-access/advisory.input.json \
  --package technical_advisory \
  --output /tmp/advisory.space.json

advisorygraphen check \
  --space /tmp/advisory.space.json \
  --ruleset technical_advisory_mvp \
  --format json \
  --output /tmp/advisory.check.report.json

advisorygraphen completions propose \
  --space /tmp/advisory.space.json \
  --from-report /tmp/advisory.check.report.json \
  --format json \
  --output /tmp/advisory.completions.json

advisorygraphen project \
  --space /tmp/advisory.space.json \
  --report /tmp/advisory.check.report.json \
  --audience executive \
  --format markdown \
  --output /tmp/executive-review.md

advisorygraphen project \
  --space /tmp/advisory.space.json \
  --report /tmp/advisory.check.report.json \
  --audience audit_trace \
  --format json \
  --output /tmp/audit-trace.json
```

## Dogfood example

AdvisoryGraphen 自身の HigherGraphen 統合判断を、同じ `technical_advisory`
パイプラインで検査する例を含めている。

```sh
advisorygraphen lift \
  --input examples/dogfood/higher-graphen-integration/advisory.input.json \
  --package technical_advisory \
  --output /tmp/advisorygraphen-dogfood.space.json

advisorygraphen check \
  --space /tmp/advisorygraphen-dogfood.space.json \
  --ruleset technical_advisory_mvp \
  --format json \
  --output /tmp/advisorygraphen-dogfood.check.json

advisorygraphen project \
  --space /tmp/advisorygraphen-dogfood.space.json \
  --report /tmp/advisorygraphen-dogfood.check.json \
  --audience audit_trace \
  --format json \
  --output /tmp/advisorygraphen-dogfood.audit.json
```

この例は、HG境界出力が受け入れテストで検証されていることと、
`higher-graphen-runtime` 採用判断が post-MVP の未検証 follow-up であることを
同じ構造モデル上で分離して扱う。

## 採用する原則

- 観測された入力は完全な真実ではない。
- AI が作った構造は、明示レビューがない限り accepted fact ではない。
- completion candidate は承認済み変更ではない。
- projection は原則として lossy であり、何を省略・圧縮したかを明示する。
- readiness、frontier、closeable は保存状態ではなく、case log から導出する。
- 顧客固有データ、商用ルール、非公開評価データは公開リポジトリに入れない。

## 含まれる成果物

- プロダクト要求
- ドメインモデル
- Rust workspace / crate 境界
- JSON schema 契約
- source adapter 設計
- invariant / obstruction / completion rule 設計
- projection 契約
- CLI 契約
- AI agent skill
- case log / storage 設計
- security / governance
- 実装ロードマップ
- テスト戦略
- ADR
- 参照入力と期待レポート
