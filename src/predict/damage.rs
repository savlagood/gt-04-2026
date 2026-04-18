#![allow(dead_code)]

use crate::geom::{chebyshev, is_in_disk};
use crate::model::params::DerivedParams;
use crate::model::state::{Construction, GameState, Plantation};

/// Fix 4: Предсказывает HP плантации на конец следующей фазы обработки.
/// Корректно обрабатывает правило `task.md` §Песчаные бури: «Буря не может
/// уничтожить плантацию, только довести HP до 1». При этом если плантацию
/// уже добивает бобёр/землетрясение — буря не «спасает», HP может уйти в минус
/// и сервер разрушит плантацию.
pub fn predict_hp_next_turn(
    p: &Plantation,
    state: &GameState,
    params: &DerivedParams,
) -> i32 {
    // Иммунитет покрывает следующий ход → без изменения.
    if p.immunity_until_turn > state.turn_no + 1 {
        return p.hp;
    }

    // Летальные источники: бобры, землетрясение.
    let mut lethal = 0i32;
    for b in &state.beavers {
        if chebyshev(p.pos, b.pos) <= 2 {
            lethal += params.beaver_dmg;
        }
    }
    for m in &state.meteo {
        if m.kind == "earthquake" && m.turns_until.unwrap_or(u32::MAX) == 0 {
            lethal += params.earthquake_dmg;
        }
    }

    let hp_after_lethal = p.hp - lethal;
    if hp_after_lethal <= 0 {
        // Летальная доза уже получена — буря ничего не добавляет и не спасает.
        return hp_after_lethal;
    }

    let storm_hits = state.meteo.iter().any(|m| {
        if m.kind != "sandstorm" {
            return false;
        }
        // Движущаяся буря (не forming). Использует next_position и radius.
        if m.forming.unwrap_or(true) {
            return false;
        }
        match (m.next_position, m.radius) {
            (Some(next), Some(r)) => is_in_disk(p.pos, next, r),
            _ => false,
        }
    });

    if storm_hits {
        (hp_after_lethal - params.storm_dmg).max(1)
    } else {
        hp_after_lethal
    }
}

/// Обёртка для прежнего интерфейса: «на сколько HP уменьшится».
pub fn predicted_damage(
    p: &Plantation,
    state: &GameState,
    params: &DerivedParams,
) -> i32 {
    p.hp - predict_hp_next_turn(p, state, params)
}

/// Fix 11: урон по недостроенной плантации.
/// По task.md: бобры бьют и строящиеся; землетрясение — «все плантации и
/// постройки». Буря по стройкам — VERIFY, пока не учитываем.
pub fn predict_construction_damage(
    c: &Construction,
    state: &GameState,
    params: &DerivedParams,
) -> i32 {
    let mut dmg = 0;
    for b in &state.beavers {
        if chebyshev(c.pos, b.pos) <= 2 {
            dmg += params.beaver_dmg;
        }
    }
    for m in &state.meteo {
        if m.kind == "earthquake" && m.turns_until.unwrap_or(u32::MAX) == 0 {
            dmg += params.earthquake_dmg;
        }
    }
    dmg
}
