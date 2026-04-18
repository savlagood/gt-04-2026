#![allow(dead_code)]

use std::time::Duration;

use crate::api::dto::{LogMessage, PlayerDTO, PlayerResponse, PublicError};
use crate::error::{BotError, Result};

#[derive(Debug, Clone, Copy)]
pub enum Server {
    Test,
    Prod,
}

impl Server {
    pub fn from_env(val: Option<&str>) -> Self {
        match val {
            Some("prod") => Server::Prod,
            _ => Server::Test,
        }
    }

    fn base_url(self) -> &'static str {
        match self {
            Server::Test => "https://games-test.datsteam.dev",
            Server::Prod => "https://games.datsteam.dev",
        }
    }
}

pub struct ApiClient {
    agent: ureq::Agent,
    base_url: String,
    token: String,
}

impl ApiClient {
    pub fn new(server: Server, token: String) -> Self {
        let agent = ureq::AgentBuilder::new()
            .timeout(Duration::from_millis(800))
            .build();
        Self {
            agent,
            base_url: server.base_url().to_string(),
            token,
        }
    }

    pub fn get_arena(&self) -> Result<PlayerResponse> {
        let url = format!("{}/api/arena", self.base_url);
        let resp = self
            .agent
            .get(&url)
            .set("X-Auth-Token", &self.token)
            .call()
            .map_err(map_ureq_err)?;
        let body: PlayerResponse = resp.into_json().map_err(BotError::Io)?;
        Ok(body)
    }

    pub fn post_command(&self, cmd: &PlayerDTO) -> Result<PublicError> {
        let url = format!("{}/api/command", self.base_url);
        let resp = self
            .agent
            .post(&url)
            .set("X-Auth-Token", &self.token)
            .send_json(serde_json::to_value(cmd)?)
            .map_err(map_ureq_err)?;
        let body: PublicError = resp.into_json().map_err(BotError::Io)?;
        Ok(body)
    }

    pub fn get_logs(&self) -> Result<Vec<LogMessage>> {
        let url = format!("{}/api/logs", self.base_url);
        let resp = self
            .agent
            .get(&url)
            .set("X-Auth-Token", &self.token)
            .call()
            .map_err(map_ureq_err)?;
        let body: Vec<LogMessage> = resp.into_json().map_err(BotError::Io)?;
        Ok(body)
    }
}

fn map_ureq_err(e: ureq::Error) -> BotError {
    match e {
        ureq::Error::Status(429, response) => {
            // Сервер подсказывает через заголовки:
            //   Retry-After         — секунды (иногда миллисекунды).
            //   X-Ratelimit-Reset   — unix-ts (сек) или секунды-до-сброса.
            //   X-Ratelimit-Remaining — сколько ещё разрешено в окне.
            let retry_after_ms = parse_retry_after(&response);
            let remaining = response.header("X-Ratelimit-Remaining").unwrap_or("?").to_string();
            let limit = response.header("X-Ratelimit-Limit").unwrap_or("?").to_string();
            let reset = response.header("X-Ratelimit-Reset").unwrap_or("?").to_string();
            tracing::debug!(
                retry_after_ms,
                remaining = %remaining,
                limit = %limit,
                reset = %reset,
                "429 headers"
            );
            BotError::RateLimited { retry_after_ms }
        }
        other => BotError::from(other),
    }
}

fn parse_retry_after(response: &ureq::Response) -> Option<u64> {
    // RFC 7231: Retry-After may be seconds or HTTP-date. Мы поддерживаем секунды.
    if let Some(v) = response.header("Retry-After") {
        if let Ok(sec) = v.trim().parse::<u64>() {
            return Some(sec * 1000);
        }
    }
    // Fallback: X-Ratelimit-Reset — количество секунд до сброса окна.
    if let Some(v) = response.header("X-Ratelimit-Reset") {
        if let Ok(sec) = v.trim().parse::<u64>() {
            // Некоторые реализации возвращают unix-ts; тогда число будет > 10^9.
            // В этом случае считаем, что сброс через n_sec = reset_ts - now_ts,
            // но без системного времени не определим. Считаем малые значения как
            // «секунды до сброса», большие игнорируем.
            if sec < 3600 {
                return Some(sec * 1000);
            }
        }
    }
    None
}
