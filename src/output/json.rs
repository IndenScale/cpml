use crate::pipeline::orchestrator::PipelineResult;

/// Print the pipeline result as pretty-printed JSON to stdout.
pub fn print_json_result(result: &PipelineResult) {
    println!(
        "{}",
        serde_json::to_string_pretty(result).expect("failed to serialize PipelineResult")
    );
}
