# DatsSol Bot

Стратегический бот для [DatsSol](https://games-test.datsteam.dev/static/datssol/openapi/) — пошаговой стратегии терраформирования планеты.

## Запуск

```sh
export DATSSOL_TOKEN=<токен>
export DATSSOL_SERVER=test   # или prod
cargo run --release
```

## Переменные окружения

| Переменная | Обязательна | Описание |
|---|---|---|
| `DATSSOL_TOKEN` | да | API-токен для `X-Auth-Token`. |
| `DATSSOL_SERVER` | нет | `test` (default) или `prod`. |
| `DATSSOL_CONFIG` | нет | Путь к TOML-конфигу стратегии, default `config.toml`. |
| `DATSSOL_LOG_DIR` | нет | Директория для per-turn JSON-логов, default `./logs`. |
| `DATSSOL_DRY_RUN` | нет | `1` → не отправлять `POST /api/command`, только логировать решение. |
| `RUST_LOG` | нет | `datssol_bot=info` (default), `=debug` — подробности, включая rate-limit headers. |

## Конфиг

Всё тюнится в `config.toml` (веса `score_cell_value`, приоритет апгрейдов, пороги безопасности, фазы по ходам). Дефолт закоммичен и встроен через `include_str!`.

## Dry run

```sh
DATSSOL_DRY_RUN=1 cargo run --release
```

## Тесты и качество

```sh
cargo test
cargo clippy --all-targets -- -D warnings
```

## Логи

- **stderr** (`RUST_LOG=datssol_bot=info`): одна строка на ход — `turn=N plan_ms=X plantations=K assignments=M post=ok|note|429|empty`.
- **per-turn JSON** в `$DATSSOL_LOG_DIR/turn_{NNNNN}.json`: полный снимок (plantations, enemies, beavers, meteo, upgrades, derived_params, predictedHp, assignments).
- **server_events.jsonl** — раз в 30 ходов тянем `GET /api/logs`, append событий (destroy, earthquake, respawn, upgrade applied).

## Постфактумная аналитика

```sh
python3 tools/analyze_round.py logs/
```

Выведет: peak/avg plantations, планирование p95, финальные апгрейды, события сервера (death_penalty, terraformed_destroy, earthquake, …), deaths per 100 turns.

## Архитектура

- `api/` — HTTP-клиент + DTO ровно по `docs/openapi.yml`.
- `model/` — нормализованный GameState, Memory, DerivedParams.
- `geom/` + `graph/` — утилиты расстояний и граф 4-связности (BFS, articulation).
- `predict/` — HP/damage, turns_until_complete, storm tracking, limit analysis.
- `tactics/` — по одному файлу на стратегию: build, repair, upgrades, main_safety (relocate), beaver, sabotage.
- `planner/` — Task/Assignment, жадный assign_tasks_mvp, plan_turn.
- `main.rs` — main loop: GET → plan → POST, синхронизация с ходом сервера (`next_turn_in + 0.15`).

## Rate-limit

Сервер — **3 rps**, но **per endpoint** (по 1 rps на `/api/arena`, `/api/command`, `/api/logs`). Бот синхронизируется с tick-ами сервера: один GET + один POST за ход (~0.95 сек). Exp-backoff на 429 с учётом `Retry-After`.
