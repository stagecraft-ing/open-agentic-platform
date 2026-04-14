# Sample use:

```shell
  crates/target/release/factory-run \
      --adapter aim-vue-node \
      --project ~/.factory-proof/example-project \
      --business-docs ~/.factory-proof/example-project.build-spec.yaml \
      --factory-root factory \
      --org example-org \
      --auto-approve \
      --scaffold-source ./upstream/aim-vue-node-template \
      --model opus \
      --extended-context \
      --thinking max \
      --step-timeout 1200 \
      --resume 4bfffe1b-6cbd-4f40-9453-a55a75679624
```