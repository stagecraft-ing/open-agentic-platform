# Sample use:

```shell
  crates/target/release/factory-run \
      --adapter aim-vue-node \
      --project ~/.factory-proof/cfs-womens-shelter \
      --business-docs ~/.factory-proof/cfs-womens-shelter.build-spec.yaml \
      --factory-root factory \
      --org goa-cfs \
      --auto-approve \
      --scaffold-source /Users/bart/Dev2/AIM-vue-node-template \
      --model claude-opus-4-6 \
      --step-timeout 1200 \
      --resume 4bfffe1b-6cbd-4f40-9453-a55a75679624
```