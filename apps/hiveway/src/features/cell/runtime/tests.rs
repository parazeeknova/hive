use super::*;
use serde_json::json;
fn limits() -> crate::runtime::SandboxLimits {
    crate::runtime::SandboxLimits {
        concurrency: 1,
        artifact_bytes: 65_536,
        input_bytes: 4096,
        output_bytes: 4096,
        schema_bytes: 4096,
        memory_bytes: 65_536,
        fuel: 1000,
        artifacts_per_wallet: 10,
        json_depth: 16,
        json_nodes: 4096,
        collection_items: 256,
    }
}
#[test]
fn offline_schemas_reject_references_and_unbounded_branching() -> anyhow::Result<()> {
    let limits = limits();
    for schema in [
        json!({"type":"object","$ref":"http://127.0.0.1/private"}),
        json!({"type":"object","$ref":"file:///etc/passwd"}),
        json!({"type":"object","oneOf":[{},{}]}),
    ] {
        assert!(schemas::compile(&schema, &limits).is_err());
    }
    let schema = schemas::compile(
        &json!({"type":"object","required":["value"],"properties":{"value":{"type":"string"}},"additionalProperties":false}),
        &limits,
    )?;
    assert!(schema.is_valid(&json!({"value":"hello"})));
    assert!(!schema.is_valid(&json!({"value":42})));
    Ok(())
}
// The interpreter and WAT compiler are exercised natively; isolated Miri covers host-side schema logic.
#[cfg(not(miri))]
#[test]
fn dynamic_json_is_processed_without_host_access() -> anyhow::Result<()> {
    let code = wat::parse_str(include_str!(
        "../../../../../../packages/local/fixtures/wasm/echo.wat"
    ))?;
    let engine = engine::engine();
    engine::verify(&engine, &code, &limits())?;
    for value in [
        json!({"message":"first request"}),
        json!({"message":"a different request","items":[1,2,3]}),
    ] {
        assert_eq!(engine::execute(&engine, &code, &value, &limits())?, value);
    }
    Ok(())
}
#[cfg(not(miri))]
#[test]
fn hostile_modules_are_bounded_and_cannot_import_host_functions() -> anyhow::Result<()> {
    let engine = engine::engine();
    for source in [
        "(module (import \"wasi_snapshot_preview1\" \"fd_write\" (func)))",
        "(module (func $start) (start $start))",
        "(module (memory (export \"memory\") 2000))",
    ] {
        assert!(engine::verify(&engine, &wat::parse_str(source)?, &limits()).is_err());
    }
    for body in [
        "(loop $forever br $forever) i64.const 0",
        "i64.const -1",
        "i64.const 999999999",
        "unreachable",
    ] {
        let source = format!(
            "(module (memory (export \"memory\") 1) (func (export \"alloc\") (param i32) (result i32) i32.const 0) (func (export \"run\") (param i32 i32) (result i64) {body}))"
        );
        let code = wat::parse_str(source)?;
        assert!(engine::execute(&engine, &code, &json!({"input":true}), &limits()).is_err());
    }
    Ok(())
}

#[test]
fn json_structure_is_bounded_before_schema_evaluation() -> anyhow::Result<()> {
    let mut limits = limits();
    data::validate(&json!({"value":[1,2,3]}), &limits, limits.input_bytes)?;
    limits.collection_items = 2;
    assert!(data::validate(&json!({"value":[1,2,3]}), &limits, limits.input_bytes).is_err());
    limits.json_depth = 1;
    assert!(
        data::validate(
            &json!({"nested":{"deep":{"value":true}}}),
            &limits,
            limits.input_bytes
        )
        .is_err()
    );
    limits.json_nodes = 2;
    assert!(data::validate(&json!({"a":1,"b":2}), &limits, limits.input_bytes).is_err());
    assert!(data::validate(&json!({"value":"oversized"}), &limits, 8).is_err());
    Ok(())
}
