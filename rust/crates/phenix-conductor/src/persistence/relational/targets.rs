fn inference_effort_token(effort: &InferenceEffort) -> &'static str {
    match effort {
        InferenceEffort::None => "none",
        InferenceEffort::Minimal => "minimal",
        InferenceEffort::Low => "low",
        InferenceEffort::Medium => "medium",
        InferenceEffort::High => "high",
        InferenceEffort::ExtraHigh => "extra_high",
        InferenceEffort::Max => "max",
    }
}
