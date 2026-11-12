# Build Report

Written during the rebuild described in `docs/REBUILD_PLAN.md`. Sections are
filled in as phases complete and finalised in Phase 8.

## Acceptance results

Pending (Phase 8).

## Measured performance

Pending (Phase 8).

## Model choice

Pending (Phase 4).

## Not exercised by CI

Pending.

## Decisions taken

- Work happens on branch `claude/optimistic-brahmagupta-lhsm71`, the branch
  that holds the plan, as the owner instructed.
- The build container cannot reach huggingface.co or github.com release
  assets (proxy policy 403). Locally the crate is compiled against the
  shared sherpa-onnx libraries from the `sherpa-onnx-core` wheel on PyPI
  (`--no-default-features --features sherpa-shared` with
  `SHERPA_ONNX_LIB_DIR`). CI builds the default static configuration.
  Everything that needs a model runs on GitHub runners only.
