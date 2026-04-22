# iggy_supporting_kafka

Reference repository for planning and implementing Kafka protocol compatibility in Apache Iggy, focused on migrating existing Kafka customers with minimal client-side changes.

## Contents

- `docs/kafka-api-keys-and-versions.md`: comprehensive Kafka API key, versioning, and compatibility design notes.
- `docs/concrete-client-implementation-matrix-and-test-plan.md`: concrete API/version priorities by client family plus executable test harness plan.
- `docs/implementation-matrix-template.md`: ready-to-fill rollout checklist and tracking matrix template.
- `docs/v1-implementation-checklist-by-module.md`: execution-ordered build checklist for Kafka facade V1.
- `docs/v1-status.md`: live status board with prioritized "start here now" tasks.
- `docs/standalone-testing-with-kafka-tools.md`: run/test instructions with Kafka and Confluent CLI tools.
- `docs/manual-testing-playbook.md`: comprehensive manual testing workflow and expected outcomes.

## Standalone Rust Code

This repository now contains a standalone Rust crate implementing checklist items:

- (1) Listener + Connection Lifecycle
- (2) Protocol Header + Codec Foundation

Key code paths:

- `src/server.rs`
- `src/protocol/header.rs`
- `src/protocol/codec.rs`
- `tests/`

## Recommended Next Steps

- Convert the API/version matrix into a tracked implementation checklist.
- Add protocol test fixtures for selected client libraries and versions.
- Keep this document updated as Iggy Kafka facade implementation evolves.
