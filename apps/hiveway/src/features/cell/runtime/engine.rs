//! A capability has no host imports: no filesystem, sockets, clocks, or environment.
use crate::{
    features::common::error::{ApiError, ApiResult},
    runtime::SandboxLimits,
};
use serde_json::Value;
use wasmi::{
    Config, EnforcedLimits, Engine, Instance, Linker, Memory, Module, Store, StoreLimits,
    StoreLimitsBuilder,
};

pub(super) fn engine() -> Engine {
    let mut config = Config::default();
    config
        .consume_fuel(true)
        .allow_start_fn(false)
        .ignore_custom_sections(true)
        .enforced_limits(EnforcedLimits::strict())
        .set_max_recursion_depth(128)
        .set_max_stack_height(1_048_576)
        .wasm_multi_memory(false);
    Engine::new(&config)
}
fn instantiate(
    engine: &Engine,
    bytes: &[u8],
    limits: &SandboxLimits,
) -> ApiResult<(Store<StoreLimits>, Instance, Memory)> {
    if bytes.len() > limits.artifact_bytes || !bytes.starts_with(b"\0asm\x01\0\0\0") {
        return Err(ApiError::bad("invalid_wasm_artifact"));
    }
    let module = Module::new(engine, bytes).map_err(|_| ApiError::bad("invalid_wasm_module"))?;
    if module.imports().next().is_some() {
        return Err(ApiError::bad("wasm_host_imports_forbidden"));
    }
    let store_limits = StoreLimitsBuilder::new()
        .memory_size(limits.memory_bytes)
        .instances(1)
        .memories(1)
        .tables(1)
        .table_elements(1024)
        .trap_on_grow_failure(true)
        .build();
    let mut store = Store::new(engine, store_limits);
    store.limiter(|limits| limits);
    store
        .set_fuel(limits.fuel)
        .map_err(|_| ApiError::unavailable("wasm_metering_unavailable"))?;
    let instance = Linker::<StoreLimits>::new(engine)
        .instantiate_and_start(&mut store, &module)
        .map_err(|_| ApiError::bad("wasm_instantiation_failed"))?;
    let memory = instance
        .get_memory(&store, "memory")
        .ok_or_else(|| ApiError::bad("wasm_memory_export_required"))?;
    instance
        .get_typed_func::<i32, i32>(&store, "alloc")
        .map_err(|_| ApiError::bad("wasm_alloc_export_required"))?;
    instance
        .get_typed_func::<(i32, i32), i64>(&store, "run")
        .map_err(|_| ApiError::bad("wasm_run_export_required"))?;
    Ok((store, instance, memory))
}
pub(super) fn verify(engine: &Engine, bytes: &[u8], limits: &SandboxLimits) -> ApiResult<()> {
    instantiate(engine, bytes, limits)?;
    Ok(())
}
pub(super) fn execute(
    engine: &Engine,
    bytes: &[u8],
    input: &Value,
    limits: &SandboxLimits,
) -> ApiResult<Value> {
    let input = serde_json::to_vec(input)?;
    if input.len() > limits.input_bytes {
        return Err(ApiError::bad("capability_input_too_large"));
    }
    let (mut store, instance, memory) = instantiate(engine, bytes, limits)?;
    let alloc = instance
        .get_typed_func::<i32, i32>(&store, "alloc")
        .map_err(|_| ApiError::bad("wasm_alloc_export_required"))?;
    let run = instance
        .get_typed_func::<(i32, i32), i64>(&store, "run")
        .map_err(|_| ApiError::bad("wasm_run_export_required"))?;
    let length =
        i32::try_from(input.len()).map_err(|_| ApiError::bad("capability_input_too_large"))?;
    let pointer = alloc
        .call(&mut store, length)
        .map_err(|_| ApiError::bad("wasm_allocation_trapped"))?;
    let offset =
        usize::try_from(pointer).map_err(|_| ApiError::bad("invalid_wasm_input_pointer"))?;
    memory
        .write(&mut store, offset, &input)
        .map_err(|_| ApiError::bad("wasm_input_out_of_bounds"))?;
    let packed = run
        .call(&mut store, (pointer, length))
        .map_err(|_| ApiError::bad("wasm_execution_trapped"))?;
    let result = u64::from_ne_bytes(packed.to_ne_bytes());
    let output_pointer =
        usize::try_from(result >> 32).map_err(|_| ApiError::bad("invalid_wasm_output_pointer"))?;
    let output_len = usize::try_from(result & 0xffff_ffff)
        .map_err(|_| ApiError::bad("invalid_wasm_output_length"))?;
    if output_len > limits.output_bytes {
        return Err(ApiError::bad("capability_output_too_large"));
    }
    let mut output = vec![0; output_len];
    memory
        .read(&store, output_pointer, &mut output)
        .map_err(|_| ApiError::bad("wasm_output_out_of_bounds"))?;
    serde_json::from_slice(&output).map_err(|_| ApiError::bad("capability_output_not_json"))
}
