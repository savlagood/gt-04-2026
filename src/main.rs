use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use datssol_bot::api::{ApiClient, Server};
use datssol_bot::config::Config;
use datssol_bot::error::BotError;
use datssol_bot::metrics::Metrics;
use datssol_bot::model::{memory::Memory, params::DerivedParams, state::GameState};
use datssol_bot::planner::{plan_turn, tasks::TaskKind, TurnPlan};
use datssol_bot::predict::predict_hp_next_turn;
use tracing_subscriber::EnvFilter;

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| "datssol_bot=info".into());
    tracing_subscriber::fmt().with_env_filter(filter).init();
}

fn main() -> anyhow::Result<()> {
    init_tracing();

    let cfg_path = std::env::var("DATSSOL_CONFIG").unwrap_or_else(|_| "config.toml".into());
    let cfg = Config::load(&cfg_path).unwrap_or_else(|e| {
        tracing::warn!("config load failed ({e}), using embedded defaults");
        Config::default_embedded()
    });

    let token = match std::env::var("DATSSOL_TOKEN") {
        Ok(t) => t,
        Err(_) => {
            tracing::error!("DATSSOL_TOKEN not set");
            return Ok(());
        }
    };
    let server = Server::from_env(std::env::var("DATSSOL_SERVER").ok().as_deref());
    let dry_run = matches!(std::env::var("DATSSOL_DRY_RUN").as_deref(), Ok("1"));
    let log_dir = PathBuf::from(
        std::env::var("DATSSOL_LOG_DIR").unwrap_or_else(|_| "logs".into()),
    );
    let _ = fs::create_dir_all(&log_dir);

    let client = ApiClient::new(server, token);
    let mut memory = Memory::default();
    let mut metrics = Metrics::default();

    // Экспоненциальный backoff на подряд идущие 429:
    //   1 → 1.5 сек, 2 → 3, 3 → 6, 4+ → 12 (cap).
    let mut consec_rate_limit: u32 = 0;
    let backoff_ms = |n: u32| -> u64 {
        let factor = 1u64 << n.clamp(1, 4).saturating_sub(1);
        1500 * factor
    };

    tracing::info!(
        server = ?server, dry_run, log_dir = %log_dir.display(),
        "main loop starting"
    );

    // Основной цикл построен вокруг ритма сервера:
    //   1) на каждом ходу делаем ровно один GET + один POST;
    //   2) в конце хода спим до начала следующего (state.next_turn_in - elapsed + offset);
    //   3) rate-limit per endpoint (1 rps) держится автоматически, так как
    //      tick сервера = 1 сек и мы делаем 1 запрос на ручку за tick;
    //   4) отдельного gate нет — он создавал бы drift (любая задержка > tick
    //      затормаживает цикл на следующих итерациях и мы выпадаем в «too late»).
    //
    // Исключения (continue без POST):
    //   - GET вернул ошибку сети / 429 → backoff и заново;
    //   - ответ сервера ещё с предыдущим turn_no (мы слишком рано проснулись) →
    //     sleep до реального начала нового хода.
    loop {
        // 1. GET arena. `got_arena_at` — точка отсчёта для sleep'а в конце tick:
        //    state.next_turn_in отсчитывается именно от момента, когда сервер
        //    отдал ответ, поэтому мерить надо от него, а не от начала итерации
        //    (tick_start включает время GET, это вносит 50 мс ошибки → мы
        //    просыпались ДО начала нового хода и видели тот же turn_no).
        let resp = match client.get_arena() {
            Ok(r) => {
                consec_rate_limit = 0;
                r
            }
            Err(BotError::RateLimited { retry_after_ms }) => {
                consec_rate_limit += 1;
                let ms = retry_after_ms.unwrap_or_else(|| backoff_ms(consec_rate_limit));
                tracing::warn!(
                    consec = consec_rate_limit,
                    backoff_ms = ms,
                    hinted = retry_after_ms.is_some(),
                    "get_arena rate limited"
                );
                metrics.api_errors += 1;
                std::thread::sleep(Duration::from_millis(ms));
                continue;
            }
            Err(e) => {
                tracing::warn!(error = %e, "get_arena failed");
                metrics.api_errors += 1;
                std::thread::sleep(Duration::from_millis(500));
                continue;
            }
        };
        let got_arena_at = Instant::now();

        // 2. Normalize. Может упасть, если turn_no=None (до регистрации).
        let state = match GameState::from_api(resp) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(error = %e, "state parse failed (retry)");
                std::thread::sleep(Duration::from_millis(500));
                continue;
            }
        };

        // 3. Повтор на один и тот же turn — мы проснулись до того, как сервер
        //    продвинул turn. Дождёмся начала нового хода.
        if memory.last_submitted_turn == Some(state.turn_no) {
            let wait = (state.next_turn_in + 0.05).max(0.05);
            std::thread::sleep(Duration::from_secs_f64(wait));
            continue;
        }

        // 4. Fix 7: respawn-детект до update.
        memory.handle_respawn_if_detected(&state);
        memory.update(&state);

        // 5. Планирование.
        let plan_start = Instant::now();
        let plan = plan_turn(&state, &memory, &cfg);
        let plan_ms = plan_start.elapsed().as_millis() as u64;
        metrics.record_plan_duration(plan_ms);
        if plan_ms > cfg.timing.plan_budget_ms {
            tracing::warn!(plan_ms, budget = cfg.timing.plan_budget_ms, "plan budget exceeded");
        }

        // 6. Запомнить стройки, которые мы начинаем.
        for a in &plan.assignments {
            if matches!(a.kind, TaskKind::Build) {
                memory.our_construction_ever.insert(a.target_pos);
            }
        }

        // 7. Per-turn JSON.
        if cfg.logging.per_turn_json {
            if let Err(e) = log_turn_minimal(&log_dir, &state, &plan, plan_ms) {
                tracing::warn!(error = %e, "turn log write failed");
            }
        }

        // 8. Отправка. Любой исход → ставим last_submitted_turn: один turn_no
        //    = максимум одна попытка POST. `post_status` — для итогового лога.
        let turn_no = state.turn_no;
        let post_status: &str;
        if plan.is_empty() {
            metrics.empty_turns += 1;
            post_status = "empty";
        } else if dry_run {
            post_status = "dry-run";
        } else {
            let dto = plan.clone().into_player_dto();
            match client.post_command(&dto) {
                Ok(pe) => {
                    consec_rate_limit = 0;
                    if pe.code != 0 {
                        tracing::warn!(code = pe.code, errors = ?pe.errors, turn = turn_no, "command response non-zero");
                        post_status = "err";
                    } else if !pe.errors.is_empty() {
                        // code=0 + errors — сервер вернул note (например `command already submitted this turn`).
                        // Это не наша ошибка в сети, а разногласие с состоянием сервера.
                        tracing::debug!(errors = ?pe.errors, turn = turn_no, "command response with notes");
                        post_status = "note";
                    } else {
                        post_status = "ok";
                    }
                }
                Err(BotError::RateLimited { retry_after_ms }) => {
                    consec_rate_limit += 1;
                    let ms = retry_after_ms.unwrap_or_else(|| backoff_ms(consec_rate_limit));
                    tracing::warn!(
                        turn = turn_no,
                        consec = consec_rate_limit,
                        backoff_ms = ms,
                        hinted = retry_after_ms.is_some(),
                        "post_command rate limited"
                    );
                    metrics.api_errors += 1;
                    std::thread::sleep(Duration::from_millis(ms));
                    post_status = "429";
                }
                Err(e) => {
                    tracing::error!(error = %e, turn = turn_no, "post_command failed");
                    metrics.api_errors += 1;
                    post_status = "fail";
                }
            }
        }
        memory.last_submitted_turn = Some(turn_no);

        tracing::info!(
            turn = turn_no,
            plan_ms,
            next_turn_in = state.next_turn_in,
            plantations = state.plantations.len(),
            assignments = plan.assignments.len(),
            upgrade = plan.upgrade.as_deref().unwrap_or(""),
            relocate = plan.relocate_main.is_some(),
            post = post_status,
            "turn"
        );

        metrics.turns_processed += 1;
        metrics.plantations_peak = metrics
            .plantations_peak
            .max(state.plantations.len() as i32);
        metrics.mains_lost = memory.mains_lost;

        if metrics.turns_processed % 10 == 0 {
            tracing::info!("{}", metrics.summary());
        }

        // Каждые 30 ходов тянем `/api/logs` — там серверные события (урон от
        // землетрясения, уничтожение плантации, респавн). Rate-limit
        // третьего endpoint'а (1 rps) нас не беспокоит: 1 запрос в 30 сек.
        if metrics.turns_processed % 30 == 0 {
            match client.get_logs() {
                Ok(logs) => {
                    let path = log_dir.join("server_events.jsonl");
                    if let Ok(mut f) = fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&path)
                    {
                        use std::io::Write as _;
                        for e in &logs {
                            let line = serde_json::json!({
                                "snapshot_turn": state.turn_no,
                                "time": e.time,
                                "message": e.message,
                            });
                            let _ = writeln!(f, "{}", line);
                        }
                    }
                    tracing::info!(entries = logs.len(), turn = state.turn_no, "server_log fetched");
                }
                Err(e) => {
                    tracing::debug!(error = %e, "get_logs failed");
                }
            }
        }

        // 9. Sleep до начала следующего хода сервера.
        //    Считаем ОТ МОМЕНТА GET-ответа (`got_arena_at`): state.next_turn_in
        //    задано именно относительно него. Offset 150 мс — гарантированный
        //    запас, чтобы проснуться ПОСЛЕ того, как сервер тикнул на новый
        //    ход (типовая длительность хода 0.9–1.0 с, нас интересует не
        //    «промахнуться на границу»).
        let elapsed_since_get = got_arena_at.elapsed().as_secs_f64();
        let to_next_turn = (state.next_turn_in - elapsed_since_get).max(0.0);
        let sleep_for = (to_next_turn + 0.15).max(0.05);
        std::thread::sleep(Duration::from_secs_f64(sleep_for));
    }
}

fn log_turn_minimal(
    dir: &std::path::Path,
    state: &GameState,
    plan: &TurnPlan,
    plan_ms: u64,
) -> std::io::Result<()> {
    let params = DerivedParams::from_state(state);
    let upgrades_tiers: Vec<_> = state
        .upgrades
        .tiers
        .iter()
        .map(|t| {
            serde_json::json!({
                "name": t.name,
                "current": t.current,
                "max": t.max,
            })
        })
        .collect();
    let derived = serde_json::json!({
        "ts": params.ts,
        "cs": params.cs,
        "rs": params.rs,
        "se": params.se,
        "be": params.be,
        "ds": params.ds,
        "mhp": params.mhp,
        "limit": params.limit,
        "sr": params.sr,
        "vr": params.vr,
        "ar": params.ar,
        "earthquakeDmg": params.earthquake_dmg,
        "beaverDmg": params.beaver_dmg,
        "stormDmg": params.storm_dmg,
    });
    let summary = serde_json::json!({
        "turnNo": state.turn_no,
        "nextTurnIn": state.next_turn_in,
        "mapSize": [state.map_size.0, state.map_size.1],
        "actionRange": state.action_range,
        "planMs": plan_ms,
        "derived": derived,
        "plantations": state.plantations.iter().map(|p| serde_json::json!({
            "id": p.id,
            "pos": p.pos.to_arr(),
            "hp": p.hp,
            "isMain": p.is_main,
            "isIsolated": p.is_isolated,
            "immunityUntilTurn": p.immunity_until_turn,
            "predictedHp": predict_hp_next_turn(p, state, &params),
        })).collect::<Vec<_>>(),
        "enemies": state.enemies.iter().map(|e| serde_json::json!({
            "id": e.id, "pos": e.pos.to_arr(), "hp": e.hp,
        })).collect::<Vec<_>>(),
        "beavers": state.beavers.iter().map(|b| serde_json::json!({
            "id": b.id, "pos": b.pos.to_arr(), "hp": b.hp,
        })).collect::<Vec<_>>(),
        "construction": state.construction.iter().map(|c| serde_json::json!({
            "pos": c.pos.to_arr(), "progress": c.progress,
        })).collect::<Vec<_>>(),
        "cellsCount": state.cells.len(),
        "mountainsCount": state.mountains.len(),
        "meteo": state.meteo.iter().map(|m| serde_json::json!({
            "kind": m.kind,
            "turnsUntil": m.turns_until,
            "id": m.id,
            "forming": m.forming,
            "position": m.position.map(|p| p.to_arr()),
            "nextPosition": m.next_position.map(|p| p.to_arr()),
            "radius": m.radius,
        })).collect::<Vec<_>>(),
        "upgrades": {
            "points": state.upgrades.points,
            "maxPoints": state.upgrades.max_points,
            "turnsUntilPoints": state.upgrades.turns_until_points,
            "tiers": upgrades_tiers,
        },
        "assignments": plan.assignments.iter().map(|a| serde_json::json!({
            "author": a.author_pos.to_arr(),
            "relay": a.relay_pos.to_arr(),
            "target": a.target_pos.to_arr(),
            "kind": kind_name(&a.kind),
            "expectedEffect": a.expected_effect,
        })).collect::<Vec<_>>(),
        "upgrade": plan.upgrade,
        "relocateMain": plan.relocate_main.as_ref().map(|v| v.iter().map(|p| p.to_arr()).collect::<Vec<_>>()),
    });
    let path = dir.join(format!("turn_{:05}.json", state.turn_no));
    fs::write(path, serde_json::to_vec_pretty(&summary)?)
}

fn kind_name(k: &TaskKind) -> &'static str {
    match k {
        TaskKind::Build => "build",
        TaskKind::Repair { .. } => "repair",
        TaskKind::Sabotage { .. } => "sabotage",
        TaskKind::BeaverAttack { .. } => "beaver_attack",
    }
}
