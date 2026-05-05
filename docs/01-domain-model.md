# 01. Domain Model

## Concept map

| AdvisoryGraphen concept | HigherGraphen mapping | Description |
| --- | --- | --- |
| `Engagement` | `Space` | ひとつの顧客案件、診断、顧問契約、レビュー単位 |
| `AdvisoryCell` | `Cell` | 主張、事実、証拠、システム、要求、リスク、施策、意思決定など |
| `AdvisoryContext` | `Context` | 部門、システム境界、ドメイン、レビュー範囲、ポリシー範囲 |
| `AdvisoryIncidence` | incidence | supports, contradicts, depends_on, owns, implements, verifies など |
| `AdvisoryMorphism` | `Morphism` | source → structure、as-is → to-be、requirement → implementation、structure → projection |
| `AdvisoryInvariant` | `Invariant` | コンサル判断が守るべき条件 |
| `AdvisoryObstruction` | `Obstruction` | 判断、実行、close が安全に進められない理由 |
| `AdvisoryCompletionCandidate` | `CompletionCandidate` | 不足証拠、追加調査、設計変更、施策、タスクの候補 |
| `AdvisoryProjection` | `Projection` | 経営者、開発者、監査、AI 向けビュー |
| `AdvisoryInterpretationPackage` | `InterpretationPackage` | consulting domain を HigherGraphen core に写像する意味レイヤー |

## Cell taxonomy

`AdvisoryCell` は、source material から直接 lift されたもの、AI が推論したもの、人間がレビューしたものを同じ型で扱う。ただし provenance と review status は必ず分ける。

| Cell type | Meaning | Example |
| --- | --- | --- |
| `observation` | bounded source snapshot から得られた観測 | 「Order Service は Billing DB を読む」 |
| `claim` | 人間またはAIによる主張 | 「この依存は将来の変更容易性を損なう」 |
| `evidence` | claim や invariant check を支える根拠 | ADR, PR, log, interview excerpt |
| `component` | システム構成要素 | iOS app, API gateway, Billing Service |
| `interface` | API, protocol, event, contract | billing status API |
| `data_store` | DB, bucket, queue, cache | Billing DB |
| `requirement` | 事業・技術・法務上の要求 | 「請求状態は注文確定時に確認する」 |
| `test_or_verification` | 検証手段 | unit test, integration test, rollout metric |
| `decision` | 採択された意思決定 | 「Billing API を新設する」 |
| `risk` | 望ましくない可能性 | 「DB schema 変更が注文処理を破壊する」 |
| `action` | 実行対象 | 「Order Service の DB 直参照を廃止する」 |
| `owner` | 責任者または責任チーム | Platform team, CTO |
| `metric` | 成功指標 | latency, incident count, delivery lead time |
| `policy` | 操作、レビュー、公開範囲の規則 | 「顧客資料は外部エクスポート禁止」 |
| `obstruction` | durable finding として保持する obstruction | 「根拠不足のため提案を確定できない」 |
| `completion` | durable candidate として保持する completion | 「追加インタビューを行う」 |
| `projection_record` | 生成された projection の記録 | executive report revision |
| `hypothesis` | obstruction を説明し得る競合的な構造仮説 | 「この越境アクセスは暗黙のクロスコンテキスト interface である」 |
| `falsifier` | hypothesis を反証し得る観測条件 | 「該当アクセスを許可する明示的な policy/ADR が存在する」 |

## Context taxonomy

| Context type | Description |
| --- | --- |
| `engagement` | 顧客案件全体の境界 |
| `business_domain` | 事業ドメインやバリューストリーム |
| `technical_boundary` | システム、サービス、モジュール、DB 所有境界 |
| `organization` | チーム、責任者、レビュー権限 |
| `policy_scope` | 公開範囲、権限、顧客データ保護、AI 利用制限 |
| `review_scope` | 今回のレビューが扱う範囲 |
| `projection_scope` | 特定の audience に出してよい情報範囲 |

## Incidence relation types

| Relation | Meaning | Hard/soft default |
| --- | --- | --- |
| `supports` | evidence が claim / decision / obstruction を支える | hard for accepted recommendation |
| `contradicts` | evidence または claim が別構造と矛盾する | hard |
| `depends_on` | action, decision, claim が別 cell に依存する | hard |
| `owns` | owner が component / action / decision を所有する | hard for action |
| `implements` | component / action が requirement を実装する | hard for requirement check |
| `verifies` | test / metric / evidence が requirement を検証する | hard for requirement check |
| `blocks` | obstruction が decision / action / close を妨げる | hard |
| `unblocks` | completion / evidence / review が obstruction を解消する | hard after review |
| `derives_from` | claim が source / evidence / derivation から導出される | soft unless policy says hard |
| `projects_to` | structure が projection に含まれる | diagnostic |
| `omits_from_projection` | projection から意図的に省略される | diagnostic, must disclose |
| `maps_to` | context-specific term or structure mapping | hard when equivalence is required |
| `explains` | hypothesis が obstruction を説明する | soft until review-promoted |
| `supported_by` | hypothesis が evidence によって支持される | soft, audited via argumentation_incidences |
| `falsified_by` | hypothesis が falsifier 観測条件によって反証され得る | soft, becomes hard if falsifier is observed |
| `competes_with` | 同じ obstruction を説明する hypothesis 同士 | diagnostic |

## Provenance and review statuses

AdvisoryGraphen では、confidence と review status を代替関係にしない。

### Evidence origin

- `source_backed`: source adapter によって取り込まれた根拠
- `inferred`: AI または heuristic による推論
- `review_promoted`: 明示レビューによって hard requirement を満たせる根拠に昇格
- `rejected`: レビューで根拠として却下されたもの
- `contradicting`: 反証または矛盾根拠

### Review status

- `unreviewed`
- `needs_review`
- `accepted`
- `rejected`
- `waived`
- `superseded`
- `reopened`

## Morphism taxonomy

| Morphism | Description |
| --- | --- |
| `source_to_advisory_space` | source snapshot を cells / contexts / incidences に lift する |
| `as_is_to_to_be` | 現状構造から提案構造への変換 |
| `requirement_to_design` | 要求が設計要素へ対応しているか |
| `design_to_verification` | 設計または要求がテスト・メトリクスで検証されているか |
| `candidate_to_accepted_structure` | completion candidate をレビュー後に構造へ反映する |
| `structure_to_projection` | executive / developer / audit / AI view へ変換する |
| `schema_v1_to_v2` | schema migration |

## Engagement lifecycle

```text
created
  -> source_snapshot_added
  -> lifted
  -> checked
  -> obstructed | ready_for_review
  -> completions_proposed
  -> review_pending
  -> candidate_accepted | candidate_rejected | evidence_required
  -> projected
  -> closed | reopened
```

Lifecycle は入力事実として記録できるが、`ready`, `blocked`, `closeable` は保存しない。これらは replay と invariant check から導出する。

## Model integrity rules

1. `AdvisorySpace` は bounded snapshot を持つ。
2. `AdvisoryCell` は stable ID を持つ。
3. `Cell`、`Context`、`Incidence`、`Morphism` は provenance を持つ。
4. AI 生成 cell の初期 `review_status` は `unreviewed` である。
5. hard requirement を満たすには、`source_backed` または `review_promoted` evidence が必要である。
6. `CompletionCandidate` を accepted structure にするには review morphism が必要である。
7. projection は represented IDs、omitted IDs、information loss を宣言する。
