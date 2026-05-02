# 00. Product Charter

## Product name

**AdvisoryGraphen**

CLI binary: `advisorygraphen`

Rust crate prefix: `advisorygraphen-*`

## One-sentence definition

AdvisoryGraphen は、コンサルティング案件を HigherGraphen の `Space`、`Cell`、`Context`、`Morphism`、`Invariant`、`Obstruction`、`CompletionCandidate`、`Projection`、`InterpretationPackage` として扱う Rust 製の構造化アドバイザリー基盤である。HigherGraphen はAIエージェントが操作する構造基盤であり、人間はprojectionとreview eventを通じて状態確認と採否判断を行う。

## Non-goal

AdvisoryGraphen は次のものではない。

- 単なる議事録管理ツール
- 汎用タスク管理ツール
- 人間向けダッシュボードだけで完結する SaaS
- AI が自動でコンサル判断を確定するシステム
- すべての顧客資料を無制限に読み込んで真実化する RAG システム

## Primary users

| User | Need | AdvisoryGraphen surface |
| --- | --- | --- |
| 技術顧問 / コンサルタント | 論点、根拠、リスク、提案、レビュー状態を管理したい | CLI, Workbench, AI view |
| 顧客経営者 | 意思決定に必要な論点だけを見たい | Executive projection |
| 開発責任者 / エンジニア | 実装可能なアクションと依存関係を見たい | Developer action projection |
| 監査 / セキュリティ / 法務 | どの主張がどの根拠に支えられているかを確認したい | Audit projection |
| AI エージェント | 追加調査、未レビュー candidate、次の安全な操作を知りたい | AI agent projection / skill |

## Operating model

AdvisoryGraphen の通常操作主体はAIエージェントである。エージェントはbounded snapshotを作成または更新し、`lift`、`check`、`completions propose`、`project ai_agent`、`project audit_trace`、`case import`、`case reason`を実行する。

人間はHG構造を直接編集する前提ではない。人間は目的、制約、判断、accept/reject/waiveなどの明示レビューを与える。AIが生成した構造やcompletion candidateは、明示レビューがない限りaccepted factではない。

## Product thesis

従来のコンサルティング成果物では、レポート本文が事実、仮説、推論、提案、意思決定を混在させやすい。AdvisoryGraphen では、レポートを最終的な真実表現としない。案件の構造を model として保持し、レポートやタスクは projection として生成する。

```text
source material
  -> bounded source snapshot
  -> advisory space
  -> cells / contexts / incidences / morphisms
  -> invariants / obstructions
  -> completion candidates
  -> evidence / provenance / review status
  -> projections
```

## MVP problem statement

MVP では、技術顧問がよく扱う次の問題に限定する。

> 顧客の技術・プロダクト・開発体制に関する資料とヒアリングを取り込み、未検証の主張、構造的な問題、意思決定に必要な不足情報、実行候補、経営者向け要約、開発者向けアクションを生成する。

## MVP success criteria

MVP は次を満たすと成功である。

1. Rust で advisory input を `AdvisorySpace` に lift できる。
2. accepted observation、AI inference、human claim、completion candidate を区別できる。
3. 少なくとも 5 つの consulting invariant を check できる。
4. invariant failure を obstruction として出力できる。
5. obstruction を解消する completion candidate を提案できる。
6. executive、developer、audit、ai_agent projection を生成できる。
7. JSON report が deterministic で、CI で snapshot test できる。
8. candidate の accept / reject をレビューイベントとして記録できる。
9. ai_agent projection が、AIエージェント向けのallowed commands、forbidden operations、resume protocol、review gateを返す。

## Product boundaries

### Public-capable layer

- Rust core types
- CLI command contract
- JSON schemas
- synthetic examples
- generic technical advisory package
- agent skill
- public documentation

### Private / commercial layer

- 顧客資料
- 実案件の case log
- 顧客固有 invariant
- 独自評価テンプレート
- commercial interpretation package
- production infrastructure
- pricing / sales / support playbook
- private benchmark data

## Initial package

最初の package は `technical_advisory` とする。

### Included

- architecture boundary review
- requirement-to-verification mapping
- ownership and responsibility checks
- action owner / success metric checks
- evidence-backed recommendation checks
- projection loss checks

### Excluded in MVP

- 自動コード解析の完全実装
- ネットワーク越しの SaaS
- 顧客ポータル UI
- LLM API 呼び出しの組み込み
- Provider-specific marketplace packaging
- Full MCP server

## Design constraints

1. CLI と JSON schema を先に安定させる。
2. Runtime workflow は deterministic にする。
3. AI が生成したものは、初期状態では `review_status = unreviewed` とする。
4. Missing evidence、conflict、obstruction はツール失敗ではなく、成功した domain finding とする。
5. Projection は必ず `projection_loss` を持つ。
6. HG構造の操作はAIエージェント向けcontractを通じて行い、人間向けUIはprojection consumerとして扱う。
