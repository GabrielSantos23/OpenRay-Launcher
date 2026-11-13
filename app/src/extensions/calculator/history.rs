use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub enum Kind {
    Math,
    UnitConversion,
    DateTime,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Calculation {
    pub id: String,
    pub expression: String,
    pub result: String,
    pub kind: Kind,
    pub created_at: u64,
    pub updated_at: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pinned: Option<u64>,
}

pub const RETENTION_MS: u64 = 92 * 86_400_000;
pub const MAX_ENTRIES: usize = 4096;
const DAY: i64 = 86_400_000;

fn path() -> std::path::PathBuf {
    crate::data_dir().join("calculator-history.json")
}

pub fn load() -> Vec<Calculation> {
    crate::store::read_json(&path())
}

pub fn save(list: &[Calculation]) -> Result<(), String> {
    crate::store::write_json(&path(), &list).map_err(|e| e.to_string())
}

pub fn update(change: impl FnOnce(Vec<Calculation>) -> Vec<Calculation>) -> Result<(), String> {
    save(&change(load()))
}

pub fn record(expression: &str, result: &str, kind: Kind) {
    if let Err(e) = update(|list| upsert(list, expression, result, kind, crate::store::now_ms()).0) {
        eprintln!("openray: could not save calculation: {e}");
    }
}

pub fn prune(list: Vec<Calculation>, now: u64) -> Vec<Calculation> {
    let kept: Vec<Calculation> = list.into_iter().filter(|c| c.pinned.is_some() || now.saturating_sub(c.updated_at) < RETENTION_MS).collect();
    let pinned_count = kept.iter().filter(|c| c.pinned.is_some()).count();
    let mut unpinned: Vec<&Calculation> = kept.iter().filter(|c| c.pinned.is_none()).collect();
    unpinned.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    let dropped: Vec<String> = unpinned.iter().skip(MAX_ENTRIES.saturating_sub(pinned_count)).map(|c| c.id.clone()).collect();
    kept.into_iter().filter(|c| !dropped.contains(&c.id)).collect()
}

fn new_id(now: u64) -> String {
    let mut bytes = [0u8; 6];
    let _ = getrandom::fill(&mut bytes);
    let base36 = |mut n: u64| {
        let mut out = Vec::new();
        loop {
            out.push(char::from_digit((n % 36) as u32, 36).unwrap_or('0'));
            n /= 36;
            if n == 0 {
                break;
            }
        }
        out.into_iter().rev().collect::<String>()
    };
    format!("{}{}", base36(now), bytes.iter().map(|b| char::from_digit((*b % 36) as u32, 36).unwrap_or('0')).collect::<String>())
}

pub fn upsert(list: Vec<Calculation>, expression: &str, result: &str, kind: Kind, now: u64) -> (Vec<Calculation>, String) {
    let trimmed = expression.trim();
    if let Some(id) = list.iter().find(|c| c.expression == trimmed).map(|c| c.id.clone()) {
        let list = list.into_iter().map(|c| if c.id == id { Calculation { result: result.into(), kind, updated_at: now, ..c } } else { c }).collect();
        return (prune(list, now), id);
    }
    let id = new_id(now);
    let mut list = list;
    list.push(Calculation { id: id.clone(), expression: trimmed.into(), result: result.into(), kind, created_at: now, updated_at: now, pinned: None });
    (prune(list, now), id)
}

pub fn pinned(list: &[Calculation]) -> Vec<Calculation> {
    let mut out: Vec<Calculation> = list.iter().filter(|c| c.pinned.is_some()).cloned().collect();
    out.sort_by_key(|c| c.pinned);
    out
}

pub fn set_pinned(list: Vec<Calculation>, id: &str, pin: bool) -> Vec<Calculation> {
    let next = list.iter().filter_map(|c| c.pinned).max().unwrap_or(0) + 1;
    list.into_iter().map(|c| if c.id == id { Calculation { pinned: pin.then_some(next), ..c } } else { c }).collect()
}

pub fn move_pinned(list: Vec<Calculation>, id: &str, step: isize) -> Vec<Calculation> {
    let order = pinned(&list);
    let Some(from) = order.iter().position(|c| c.id == id) else { return list };
    let to = from as isize + step;
    if to < 0 || to as usize >= order.len() {
        return list;
    }
    let (a, b) = (order[from].clone(), order[to as usize].clone());
    list.into_iter()
        .map(|c| match &c.id {
            x if *x == a.id => Calculation { pinned: b.pinned, ..c },
            x if *x == b.id => Calculation { pinned: a.pinned, ..c },
            _ => c,
        })
        .collect()
}

pub fn search(list: &[Calculation], query: &str) -> Vec<Calculation> {
    let q = query.trim().to_lowercase();
    list.iter().filter(|c| q.is_empty() || c.expression.to_lowercase().contains(&q) || c.result.to_lowercase().contains(&q)).cloned().collect()
}

pub fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = year + (month - 1).div_euclid(12);
    let month = (month - 1).rem_euclid(12) + 1;
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

pub fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + i64::from(month <= 2), month, day)
}

pub fn group(list: &[Calculation], now: u64, offset_ms: i64) -> Vec<(&'static str, Vec<Calculation>)> {
    let local_day = |t: u64| (t as i64 + offset_ms).div_euclid(DAY);
    let today = local_day(now);
    let weekday = (today + 3).rem_euclid(7);
    let (year, month, _) = civil_from_days(today);
    let bounds = [("Today", today), ("Yesterday", today - 1), ("This Week", today - weekday), ("This Month", days_from_civil(year, month, 1)), ("This Year", days_from_civil(year, 1, 1)), ("Older", i64::MIN)];
    let mut recent: Vec<&Calculation> = list.iter().filter(|c| c.pinned.is_none()).collect();
    recent.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    let mut sections: Vec<(&'static str, Vec<Calculation>)> = std::iter::once(("Pinned", pinned(list))).chain(bounds.iter().map(|(title, _)| (*title, Vec::new()))).collect();
    for c in recent {
        let day = local_day(c.updated_at);
        let slot = bounds.iter().position(|(_, start)| day >= *start).unwrap_or(bounds.len() - 1);
        sections[slot + 1].1.push(c.clone());
    }
    sections.into_iter().filter(|(_, items)| !items.is_empty()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(y: i64, m: i64, d: i64, h: i64) -> u64 {
        (days_from_civil(y, m, d) * DAY + h * 3_600_000) as u64
    }

    fn now() -> u64 {
        at(2026, 10, 21, 15)
    }

    #[test]
    fn upserts_by_expression_and_keeps_the_newest_result() {
        let (list, id) = upsert(Vec::new(), " 2+2 ", "4", Kind::Math, now() - 1000);
        let (list, _) = upsert(list, "10 usd in eur", "€9.20", Kind::UnitConversion, now() - 500);
        let (list, again) = upsert(list, "2+2", "4", Kind::Math, now());
        assert_eq!(again, id);
        assert_eq!(list.len(), 2);
        let entry = list.iter().find(|c| c.id == id).unwrap();
        assert_eq!((entry.expression.as_str(), entry.created_at, entry.updated_at), ("2+2", now() - 1000, now()));
    }

    #[test]
    fn drops_unpinned_entries_older_than_three_months() {
        let old = Calculation { id: "a".into(), expression: "1+1".into(), result: "2".into(), kind: Kind::Math, created_at: 0, updated_at: now() - RETENTION_MS - 1, pinned: None };
        let kept = prune(vec![old.clone(), Calculation { id: "b".into(), pinned: Some(1), ..old }], now());
        assert_eq!(kept.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(), ["b"]);
    }

    #[test]
    fn pins_in_order_and_moves_pinned_entries() {
        let (mut list, _) = upsert(Vec::new(), "1", "1", Kind::Math, now());
        let mut ids = Vec::new();
        for (i, e) in ["a", "b", "c"].iter().enumerate() {
            let (next, id) = upsert(list, e, e, Kind::Math, now() + i as u64);
            list = next;
            ids.push(id);
        }
        for id in &ids {
            list = set_pinned(list, id, true);
        }
        list = move_pinned(list, &ids[2], -1);
        assert_eq!(group(&list, now(), 0)[0].1.iter().map(|c| c.expression.as_str()).collect::<Vec<_>>(), ["a", "c", "b"]);
        list = set_pinned(list, &ids[0], false);
        assert_eq!(group(&list, now(), 0).iter().map(|s| s.0).collect::<Vec<_>>(), ["Pinned", "Today"]);
    }

    #[test]
    fn groups_by_day_week_month_year_and_older() {
        let entry = |id: &str, t: u64| Calculation { id: id.into(), expression: id.into(), result: id.into(), kind: Kind::Math, created_at: t, updated_at: t, pinned: None };
        let list = vec![entry("today", now() - 1000), entry("yesterday", now() - DAY as u64), entry("week", at(2026, 10, 19, 9)), entry("month", at(2026, 10, 2, 0)), entry("year", at(2026, 2, 1, 0)), entry("older", at(2025, 12, 1, 0))];
        let groups: Vec<(&str, String)> = group(&list, now(), 0).into_iter().map(|(t, items)| (t, items[0].id.clone())).collect();
        let expected = [("Today", "today"), ("Yesterday", "yesterday"), ("This Week", "week"), ("This Month", "month"), ("This Year", "year"), ("Older", "older")];
        assert_eq!(groups, expected.map(|(t, id)| (t, id.to_string())));
    }

    #[test]
    fn searches_expressions_and_results() {
        let (list, _) = upsert(Vec::new(), "15% of 80", "12", Kind::Math, now());
        assert_eq!(search(&list, "12").len(), 1);
        assert_eq!(search(&list, "of 8").len(), 1);
        assert_eq!(search(&list, "usd").len(), 0);
    }

    #[test]
    fn civil_round_trip() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(civil_from_days(days_from_civil(2026, 10, 21)), (2026, 10, 21));
        assert_eq!((days_from_civil(2026, 10, 21) + 3).rem_euclid(7), 2);
    }
}
