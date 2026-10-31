//! Bounded OpenAI-compatible JSON/SSE responses; reasoning is never retained.
use crate::features::common::error::{ApiError, ApiResult};
use serde_json::{Value, json};
#[derive(Default)]
struct Completion {
    pending: Vec<u8>,
    content: String,
    stopped: bool,
}
impl Completion {
    fn line(&mut self, line: &[u8]) -> ApiResult<()> {
        let Some(data) = line.strip_prefix(b"data:") else {
            return Ok(());
        };
        let data = data.trim_ascii();
        if data == b"[DONE]" {
            return Ok(());
        }
        let value: Value =
            serde_json::from_slice(data).map_err(|_| ApiError::bad("invalid_builder_response"))?;
        if value.get("error").is_some() {
            return Err(ApiError::unavailable("builder_provider_rejected"));
        }
        if let Some(part) = value["choices"][0]["delta"]["content"].as_str() {
            if self.content.len() + part.len() > 131_072 {
                return Err(ApiError::bad("builder_response_too_large"));
            }
            self.content.push_str(part);
        }
        if value["choices"][0]["finish_reason"] == "stop" {
            self.stopped = true;
        }
        Ok(())
    }
    fn push(&mut self, bytes: &[u8]) -> ApiResult<()> {
        self.pending.extend_from_slice(bytes);
        while let Some(end) = self.pending.iter().position(|b| *b == b'\n') {
            let line: Vec<u8> = self.pending.drain(..=end).collect();
            self.line(line.trim_ascii())?;
        }
        if self.pending.len() > 131_072 {
            return Err(ApiError::bad("builder_response_too_large"));
        }
        Ok(())
    }
    fn finish(mut self) -> ApiResult<Value> {
        let remaining = std::mem::take(&mut self.pending);
        self.line(remaining.trim_ascii())?;
        if !self.stopped {
            return Err(ApiError::bad("builder_generation_incomplete"));
        }
        Ok(json!({"choices":[{"finish_reason":"stop","message":{"content":self.content}}]}))
    }
}
pub(super) fn network_error(error: &reqwest::Error) -> ApiError {
    ApiError::unavailable(if error.is_timeout() {
        "builder_provider_timeout"
    } else {
        "builder_provider_unavailable"
    })
}
pub(super) async fn read(mut response: reqwest::Response) -> ApiResult<Value> {
    let streaming = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.starts_with("text/event-stream"));
    let mut completion = Completion::default();
    let mut json = Vec::new();
    let mut total = 0;
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| network_error(&error))?
    {
        total += chunk.len();
        if total > 8 * 1024 * 1024 {
            return Err(ApiError::bad("builder_response_too_large"));
        }
        if streaming {
            completion.push(&chunk)?;
        } else {
            if json.len() + chunk.len() > 131_072 {
                return Err(ApiError::bad("builder_response_too_large"));
            }
            json.extend_from_slice(&chunk);
        }
    }
    if streaming {
        completion.finish()
    } else {
        serde_json::from_slice(&json).map_err(|_| ApiError::bad("invalid_builder_response"))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fragmented_sse_ignores_reasoning_and_requires_complete_generation() {
        let stream=b"data: {\"choices\":[{\"delta\":{\"reasoning_content\":\"private reasoning\"}}]}\n\ndata: {\"choices\":[{\"delta\":{\"content\":\"{}\"}}]}\n\ndata: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n";
        let mut completion = Completion::default();
        for chunk in stream.chunks(7) {
            assert!(completion.push(chunk).is_ok());
        }
        let value = completion.finish();
        assert!(value.is_ok());
        assert_eq!(
            value
                .ok()
                .map(|v| v["choices"][0]["message"]["content"].clone()),
            Some(json!("{}"))
        );
        assert!(Completion::default().finish().is_err());
    }
}
