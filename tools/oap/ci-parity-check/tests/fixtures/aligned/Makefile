# Aligned fixture Makefile — every workflow token has a local mirror.
ci-axiomregent:
	cargo test --manifest-path crates/axiomregent/Cargo.toml

factory-schema-lockstep:
	./tools/oap/factory-schema-lockstep/target/release/factory-schema-lockstep --oap-dir standards/schemas/factory --factory-dir .factory-encore/contract/schemas
