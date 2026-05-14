# AdvisoryGraphen

AdvisoryGraphen は、HigherGraphen 上で根拠に基づくコンサルティング / アドバイザリーワークフローと提案ガバナンスを扱う Rust 製 CLI です。

戦略メモ、運用制約、アーキテクチャメモ、ADR、Issue 要約、ヒアリングメモ、要求、検証記録などの bounded source material を構造化された advisory space に変換します。その構造から obstruction を検出し、レビュー可能な completion candidate を提案し、append-only な case history を保持し、audience ごとの projection を生成します。

リリース済み CLI は、意図的にファイルベースかつ deterministic なツールとして提供しています。Hosted SaaS、汎用タスク管理ツール、人間のレビューなしにコンサル判断を確定する AI システムではありません。

最初のリリースで同梱している interpretation package は `technical_advisory_mvp` で、技術顧問、アーキテクチャレビュー、プロダクト / 開発意思決定支援にフォーカスしています。ただし基盤モデル自体はより広く、領域固有の interpretation package、invariant、projection policy を追加することで他のコンサルティング領域にも適用できます。

## インストール

crates.io の配布パッケージ名は `advisorygraphen-cli` です。インストール後の実行コマンドは `advisorygraphen` です。

```sh
cargo install advisorygraphen-cli
advisorygraphen version
```

現在のリリース: `0.1.1`

このリポジトリからローカル実行する場合:

```sh
cargo run -q -p advisorygraphen-cli -- version
```

## Quick Start

同梱の `technical_advisory_mvp` fixture で、リリース済みワークフローを実行します。

```sh
advisorygraphen validate \
  --input examples/technical-advisory/direct-db-access/advisory.input.json \
  --format json

advisorygraphen lift \
  --input examples/technical-advisory/direct-db-access/advisory.input.json \
  --package technical_advisory \
  --output /tmp/advisory.space.json \
  --format json

advisorygraphen check \
  --space /tmp/advisory.space.json \
  --ruleset technical_advisory_mvp \
  --output /tmp/advisory.check.report.json \
  --format json

advisorygraphen completions propose \
  --space /tmp/advisory.space.json \
  --from-report /tmp/advisory.check.report.json \
  --output /tmp/advisory.completions.report.json \
  --format json

advisorygraphen project \
  --space /tmp/advisory.space.json \
  --report /tmp/advisory.check.report.json \
  --completions-report /tmp/advisory.completions.report.json \
  --audience executive \
  --format markdown \
  --output /tmp/executive-review.md
```

AI エージェント運用では、再開プロトコルとして `ai_agent` projection を生成します。

```sh
advisorygraphen project \
  --space /tmp/advisory.space.json \
  --report /tmp/advisory.check.report.json \
  --completions-report /tmp/advisory.completions.report.json \
  --audience ai_agent \
  --format json \
  --output /tmp/ai-agent.json
```

## Core Workflow

AdvisoryGraphen は次のモデルで動作します。

```text
bounded source snapshot
  -> advisory space
  -> invariant check report
  -> reviewable completion candidates
  -> audience-specific projections
  -> append-only case log and case reasoning
```

主要コマンド:

| Command | Purpose |
| --- | --- |
| `validate` | snapshot、advisory space、report、projection request、review event を検証する。 |
| `lift` | bounded source snapshot を advisory space に変換する。 |
| `check` | advisory invariant を評価し、obstruction を出力する。 |
| `completions propose` | obstruction からレビュー可能な completion candidate を生成する。 |
| `completions dry-run` | case store を変更せず、candidate をメモリ上で適用して再チェックする。 |
| `project` | audience ごとの projection を生成する。 |
| `case import` | space を append-only なローカル case store に取り込む。 |
| `case reason` | case state を replay し、readiness、blocker、frontier、close status を導出する。 |
| `case close-check` | 指定 revision で case を close できるか確認する。 |
| `hypothesis propose` | source-backed signal から、レビュー可能な hypothesis lifecycle transition を提案する。 |
| `observation record` | hypothesis を support/falsify する前に bounded observation result を記録する。 |
| `dogfood repo-snapshot` | このリポジトリ自身の自己レビュー用 bounded snapshot を生成する。 |
| `code repo-snapshot` | コード由来の advisory signal を得るための bounded lexical code snapshot を生成する。 |

現在の CLI surface は `advisorygraphen --help` または `advisorygraphen <command> --help` で確認できます。

## Core Ideas

AdvisoryGraphen は、次の考え方を中心に設計しています。

- レポートは projection であり、source of truth ではありません。Source of truth は、構造化された advisory space と append-only な case log です。
- Source material は bounded input であり、自動的に真実化されるものではありません。Claim、evidence、AI inference、reviewed conclusion は分離して扱います。
- Advisory work は conclusion-first ではなく hypothesis-first で進めます。Proposal は hypothesis と observation から導出し、未支持の hypothesis は見える状態に残します。
- AI が生成した structure は review-gated です。Candidate は有用で具体的で source-backed でも、review なしに accepted にはなりません。
- Obstruction は第一級の consulting object です。意思決定、提案、アーキテクチャ、計画、case close を安全に進めるうえで何が妨げになっているかを表します。
- Projection loss は明示します。Audience-specific summary は、何を省略し、圧縮し、証明できないまま残しているかを開示して初めて安全に使えます。
- Domain knowledge は interpretation package に置きます。`technical_advisory_mvp` は最初の package であり、モデルの限界ではありません。

つまり AdvisoryGraphen は、コンサルティング業務を単一のレポート文書としてではなく、evidence、hypothesis、constraint、obstruction、proposal、review event、projection の構造として扱います。

## Projection Audiences

`project` は次の audience をサポートします。

| Audience | Use |
| --- | --- |
| `executive` | 経営・意思決定向けの要約。 |
| `developer_action` | 実装担当者向けの action と blocker。 |
| `audit_trace` | evidence、provenance、review status、projection loss。 |
| `ai_agent` | agent resume protocol、allowed command、forbidden operation、review gate、observation action、残 blocker。 |
| `client_review` | client-facing な executive-style review。 |
| `cli` | CLI 向けの executive-style view。 |

Projection は意図的に lossy な view です。represented ID、omitted information、projection loss metrics、schema morphisms、review state を確認する場合は `audit_trace` または `ai_agent` を使ってください。

## Review-Gated Advisory

AdvisoryGraphen は、fact、claim、AI inference、hypothesis、obstruction、completion candidate、review event、projection を分離して扱います。

Completion candidate は提案です。明示的な review event と対応する application step がない限り、accepted structure ではありません。Accepted completion review と、candidate を advisory space に materialize することも別の操作です。

重要な review command:

```sh
advisorygraphen completions accept \
  --store .advisorygraphen/store \
  --candidate-id candidate:example \
  --from-report /tmp/advisory.completions.report.json \
  --reviewer reviewer:cto \
  --reason "Accepted target direction" \
  --base-revision revision:technical-advisory-smoke-1 \
  --format json

advisorygraphen completions apply-accepted \
  --store .advisorygraphen/store \
  --space-id space:advisory:technical-advisory-direct-db-access \
  --reviewer ai-agent:codex \
  --reason "Apply reviewed accepted completion candidates" \
  --base-revision revision:review-000002 \
  --format json
```

`ai_agent` projection と `case reason` output は、エージェントの operational contract として扱います。これらは review gate、observation action、candidate review state、blocker resolution requirement、projection loss metrics、安全な次コマンドを公開します。

## Code-Derived Snapshot

`code repo-snapshot` adapter は、ローカルリポジトリから bounded lexical snapshot を生成します。初期 adapter は `package.json`、`tsconfig.json`、API route file、test file、database access pattern、`process.env.*` usage など、deterministic な TypeScript、JavaScript、Next.js signal を対象にします。

```sh
advisorygraphen code repo-snapshot \
  --repo . \
  --output /tmp/advisory-code.input.json \
  --format json
```

この adapter は意図的に conservative です。TypeScript type や runtime control flow は解決しません。Lexical adapter が証明できない evidence は observation と review event で記録してください。

## Public and Private Boundary

公開リリースに含まれるもの:

- Rust core types と CLI workflow。
- Stable JSON schemas。
- 最初の技術顧問ユースケース向けの generic `technical_advisory_mvp` package behavior。
- Synthetic examples と dogfood fixtures。
- Public documentation と agent skill。

公開リポジトリや公開 package に含めないもの:

- 顧客 source material と実案件 case log。
- 顧客固有 invariant や interpretation package。
- Commercial template、private benchmark、pricing、sales、support playbook。
- Production infrastructure や hosted-service secret。

実顧客データを examples、fixtures、projections、case logs として公開しないでください。

## Repository Map

| Path | Purpose |
| --- | --- |
| `tools/advisorygraphen-cli` | `advisorygraphen` command-line binary。 |
| `crates/advisorygraphen-core` | Shared DTO、ID、validation、report envelope、error policy。 |
| `crates/advisorygraphen-lift` | Snapshot-to-space lift workflow。 |
| `crates/advisorygraphen-reasoning` | Invariant check、obstruction、completion、close status。 |
| `crates/advisorygraphen-projection` | Executive、developer、audit、AI projection。 |
| `crates/advisorygraphen-runtime` | File workflow と local append-only case store。 |
| `schemas/advisorygraphen` | JSON schema contract。 |
| `examples` | Synthetic advisory fixture と dogfood fixture。 |
| `skills/advisorygraphen` | Agent-facing operating guidance。 |
| `docs` | Product、domain、CLI、storage、security、testing documentation。 |
| `adrs` | Architecture decision records。 |

完全なドキュメント一覧は `MANIFEST.md` を参照してください。

## Development

主要なローカルチェック:

```sh
cargo fmt --all --check
cargo check --workspace
cargo clippy --workspace --all-targets
cargo test --workspace
cargo test --manifest-path tests/advisorygraphen-cli-acceptance/Cargo.toml
```

Release packaging check:

```sh
cargo package --workspace
```

## More Documentation

- `CHANGELOG.md`: リリース履歴。
- `docs/00-product-charter.md`: product purpose、users、boundaries、non-goals。
- `docs/03-data-contracts.md`: JSON schema と report contract。
- `docs/05-reasoning-invariants.md`: invariant と obstruction policy。
- `docs/06-completion-and-review-workflow.md`: completion candidate lifecycle。
- `docs/07-projections.md`: projection contract と projection loss。
- `docs/08-cli-contract.md`: full CLI contract と exit code policy。
- `docs/09-agent-integration-and-skill.md`: agent operation model。
- `docs/10-storage-case-log.md`: append-only case log design。
- `docs/11-security-governance.md`: data と projection governance。
