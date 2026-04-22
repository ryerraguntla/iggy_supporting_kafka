# iggy_supporting_kafka

Reference repository for planning and implementing Kafka protocol compatibility in Apache Iggy, focused on migrating existing Kafka customers with minimal client-side changes.

## Contents

- `docs/kafka-api-keys-and-versions.md`: comprehensive Kafka API key, versioning, and compatibility design notes.
- `docs/concrete-client-implementation-matrix-and-test-plan.md`: concrete API/version priorities by client family plus executable test harness plan.
- `docs/implementation-matrix-template.md`: ready-to-fill rollout checklist and tracking matrix template.
- `docs/v1-implementation-checklist-by-module.md`: execution-ordered build checklist for Kafka facade V1.
- `docs/v1-status.md`: live status board with prioritized "start here now" tasks.

## Recommended Next Steps

- Convert the API/version matrix into a tracked implementation checklist.
- Add protocol test fixtures for selected client libraries and versions.
- Keep this document updated as Iggy Kafka facade implementation evolves.
