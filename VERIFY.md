# VERIFY — открытые вопросы, требующие проверки на test-сервере

Каждый пункт — помечен номером Fix (из revised plan) или VQ (verify question).
Статус: OPEN / CONFIRMED / REVISED.

| # | Вопрос | Статус | Как проверить | Текущее допущение |
|---|---|---|---|---|
| VQ1 (Fix 3) | Влияет ли `repair_power` на CS? | OPEN | После первой покупки `repair_power` замерить turns_to_complete стройки на пустой клетке единственным автором. CS ≈ 50/turns. | CS = 5 (константа), не зависит от repair_power. |
| VQ2 (Fix 1) | Награда за логово: `20 * base` или `20` или `base`? | OPEN | После первого убийства логова снять прирост очков в логах и сверить с `20 * cell_base_points(pos)`. | reward = 20 * base_points. |
| VQ3 | `signal_range` max level? | **CONFIRMED: max=10** (2026-04-18 dry-run log turn 2 — tier `signal_range` max=10) | — | Обновить priority_order в конфиге (можно брать этот апгрейд несколько раз). |
| VQ4 (Fix 11) | Бьёт ли буря по construction? | OPEN | В ходах с бурей сверить прогресс и урон по нашим активным стройкам в её disk. | Нет — не учитываем. |
| VQ5 | GET /api/arena до регистрации — что приходит? | **CONFIRMED**: turn_00000.json показывает пустое состояние (actionRange=0, mapSize=(0,0), plantations=[]). Обработано через retry в main loop. | — | OK |
| VQ6 (Fix 7) | При relocateMain меняется ли id ЦУ? | **CONFIRMED: main_id СМЕНЯЕТСЯ**: server_events показал «Spawned MAIN at [to_pos]» после relocate. В нашей memory это могло ловиться как респавн — фикс: требуем, чтобы new_main_id НЕ был в `birth_turn` (иначе это известный сосед, relocate). | — | OK |
| VQ7 | Storm `position` при `forming=true` — присутствует? | OPEN | Найти в логах буру в форминге. | `predict_storm` отсекает forming=true, так что без разницы. |
| VQ8 | Порядок конкурирующих диверсий в одном ходу | OPEN | В live-логах при парной диверсии сверить очки. | Как в task.md: кто нанёс больше в последний ход — тому очки. |

## Action log

- 2026-04-18: Файл создан. Все пункты OPEN.
- 2026-04-18 dry-run #2: VQ3 CONFIRMED (signal_range.max=10). Замечено: max_hp.max=5 (ок), settlement_limit.max=10 (ок), beaver_damage_mitigation.max=5 (ок).
- 2026-04-18 наблюдение: между раундами сервер присылает `plantations=[]` (mapSize сменился с 298×298 на 258×258). MVP не может выдать команду → `empty command`. Нужен fallback через upgrade-покупку когда есть points.
- 2026-04-18 наблюдение: `immunityUntilTurn` — абсолютный номер хода (на turn 2 iUT=4 → иммунитет до хода 4 включительно).
- 2026-04-18 rate-limit: `X-Ratelimit-Limit: 3` — **per endpoint** (1 rps на /api/arena, /api/command, /api/logs). Синхронизация с `next_turn_in` держит ритм; backoff по `Retry-After` или exp (1.5→3→6→12 сек).
- 2026-04-18 tick: **длительность хода ≈ 0.95 сек** (по изменениям `next_turn_in` между логами). Не ровно 1 сек.
- 2026-04-18 VQ5: ответ до регистрации = пустое состояние (`actionRange=0`, `plantations=[]`). Не ошибка; просто пустышка, retry не нужен — `GameState::from_api` отклоняет через `turnNo=None`.
- 2026-04-18 VQ6: при relocateMain main_id СМЕНЯЕТСЯ на id бывшего соседа. Мой детект респавна был ложным в этом случае → исправлено (требуем, чтобы new_main_id не был известен ранее).
- 2026-04-18 баг: сервер возвращает `Invalid plantation action: invalid target [X,Y]` для клеток с `terraformation_progress >= 100` (до 30-ходового decay). Фикс: исключать такие в `find_buildable_cells`.
