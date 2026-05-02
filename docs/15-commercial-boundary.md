# 15. Commercial and Public Boundary

## Principle

AdvisoryGraphen should use an open core and a protected commercial/private layer.

## Public-capable artifacts

The following may live in a public repository when they contain no customer data:

- Rust core crates
- CLI tool
- stable schemas
- synthetic examples
- generic `technical_advisory_mvp` package
- skill file
- docs
- public ADRs
- public fixtures

## Private/commercial artifacts

The following should remain private unless intentionally published:

- customer-specific engagement snapshots
- customer advisory spaces
- actual reports
- case logs
- support notes
- deployment runbooks
- proprietary interpretation packages
- private evaluation datasets
- prompt/evaluation corpora
- commercial pricing and sales strategy
- partner terms
- secrets and credentials

## Interpretation package policy

A package can be public when:

- it is generic;
- it contains no customer-specific rule;
- it is meant as a reference implementation;
- it has synthetic examples only.

A package should be private when:

- it encodes proprietary consulting method;
- it contains customer-specific vocabulary;
- it contains private risk scoring;
- it contains sensitive operational workflow;
- it creates competitive advantage.

## Repository split

Recommended:

```text
advisorygraphen-public/
  core crates
  cli
  schemas
  generic package
  examples
  docs

advisorygraphen-commercial/
  production packages
  hosted services
  customer adapters
  private projections
  deployment runbooks

customer-workspaces/
  customer-specific spaces
  case logs
  reports
  source snapshots
```

## Publication checklist

Before committing any file to a public repository:

1. no customer name or identifiable content;
2. no real architecture or incident details;
3. no secrets;
4. no private pricing;
5. no proprietary package rules unless intentionally open;
6. no raw prompts or eval corpora that expose customer material;
7. source examples are synthetic;
8. `.advisorygraphen/store` and runtime artifacts are ignored.

## `.gitignore` recommendations

```gitignore
.advisorygraphen/
*.local.json
*.customer.json
*.secret.json
reports/private/
customer-workspaces/
.env
.env.*
```

## Hosted service boundary

Hosted execution should be developed after CLI stability. It must not change the conceptual model. Hosted service is a deployment and operations layer over the same structures, logs, schemas, and projection rules.
