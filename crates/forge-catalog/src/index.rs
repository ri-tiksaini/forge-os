#![forbid(unsafe_code)]

//! Trigram inverted index + integer-only ranking + bounded Levenshtein fallback (Blueprint 5.2 & 5.3).

use std::collections::{BTreeMap, HashSet};

use crate::model::CatalogEntry;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchHit {
    pub index: usize,
    pub id: String,
    pub name: String,
    pub score: i32,
}

#[derive(Debug, Clone)]
pub struct CatalogIndex {
    entries: Vec<CatalogEntry>,
    id_hashes: Vec<(u64, usize)>,
    trigram_table: Vec<(u32, u32, u32)>, // (key, offset, len)
    postings: Vec<u32>,
}

impl CatalogIndex {
    pub fn build(mut entries: Vec<CatalogEntry>) -> Self {
        entries.sort_by(|a, b| a.id.cmp(&b.id));

        let mut id_hashes: Vec<(u64, usize)> = entries
            .iter()
            .enumerate()
            .map(|(i, e)| (fx_hash(&e.id.to_ascii_lowercase()), i))
            .collect();
        id_hashes.sort_by_key(|&(h, _)| h);

        let mut gram_map: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
        for (idx, e) in entries.iter().enumerate() {
            let mut seen = HashSet::new();
            let mut add_str = |s: &str| {
                let norm = normalize(s);
                for g in extract_trigrams(&norm) {
                    if seen.insert(g) {
                        gram_map.entry(g).or_default().push(idx as u32);
                    }
                }
            };
            add_str(&e.id);
            add_str(&e.name);
            for kw in &e.keywords {
                add_str(kw);
            }
        }

        let mut trigram_table = Vec::with_capacity(gram_map.len());
        let mut postings = Vec::new();
        for (key, list) in gram_map {
            let off = postings.len() as u32;
            let len = list.len() as u32;
            postings.extend(list);
            trigram_table.push((key, off, len));
        }

        Self {
            entries,
            id_hashes,
            trigram_table,
            postings,
        }
    }

    pub fn entries(&self) -> &[CatalogEntry] {
        &self.entries
    }

    /// O(log n) lookup by lowercase id via sorted id_hash (Blueprint 5.1).
    pub fn get(&self, id: &str) -> Option<&CatalogEntry> {
        let target = id.to_ascii_lowercase();
        let h = fx_hash(&target);
        let pos = self.id_hashes.partition_point(|&(k, _)| k < h);
        for &(k, idx) in &self.id_hashes[pos..] {
            if k != h {
                break;
            }
            if self.entries[idx].id.eq_ignore_ascii_case(&target) {
                return Some(&self.entries[idx]);
            }
        }
        None
    }

    /// Blueprint 5.2 + 5.3 deterministic integer-only search.
    pub fn search(
        &self,
        raw_query: &str,
        limit: usize,
        device_tier: u8,
        installed_ids: &HashSet<String>,
    ) -> Vec<SearchHit> {
        let q = normalize(raw_query);
        if q.is_empty() {
            return self
                .entries
                .iter()
                .enumerate()
                .take(limit)
                .map(|(i, e)| SearchHit {
                    index: i,
                    id: e.id.clone(),
                    name: e.name.clone(),
                    score: i32::from(e.popularity),
                })
                .collect();
        }

        let grams = extract_trigrams(&q);
        let mut candidates: Vec<u32> = if q.len() < 3 {
            self.entries
                .iter()
                .enumerate()
                .filter(|(_, e)| {
                    normalize(&e.id).starts_with(&q) || normalize(&e.name).starts_with(&q)
                })
                .map(|(i, _)| i as u32)
                .collect()
        } else {
            let mut lists: Vec<&[u32]> = Vec::with_capacity(grams.len());
            let mut missing_gram = false;
            for &g in &grams {
                match self.postings_for(g) {
                    Some(slice) => lists.push(slice),
                    None => {
                        missing_gram = true;
                        break;
                    }
                }
            }
            if missing_gram || lists.is_empty() {
                Vec::new()
            } else {
                lists.sort_by_key(|l| l.len());
                intersect_sorted(&lists)
            }
        };

        // Fallback on miss: bounded Levenshtein distance <= 2 (Blueprint 5.2)
        if candidates.is_empty() {
            for (i, e) in self.entries.iter().enumerate() {
                let id_norm = normalize(&e.id);
                let name_norm = normalize(&e.name);
                if levenshtein_le2(&q, &id_norm) || levenshtein_le2(&q, &name_norm) {
                    candidates.push(i as u32);
                }
            }
        }

        let mut scored: Vec<SearchHit> = candidates
            .into_iter()
            .map(|idx| {
                let e = &self.entries[idx as usize];
                let score = score_entry(e, &q, &grams, device_tier, installed_ids.contains(&e.id));
                SearchHit {
                    index: idx as usize,
                    id: e.id.clone(),
                    name: e.name.clone(),
                    score,
                }
            })
            .collect();

        // Deterministic sort: descending score, tie-break ascending lexicographic id
        scored.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.id.cmp(&b.id)));
        scored.truncate(limit);
        scored
    }

    fn postings_for(&self, gram: u32) -> Option<&[u32]> {
        let pos = self
            .trigram_table
            .binary_search_by_key(&gram, |&(k, _, _)| k)
            .ok()?;
        let (_, off, len) = self.trigram_table[pos];
        let start = off as usize;
        let end = start + len as usize;
        Some(&self.postings[start..end])
    }
}

/// Deterministic integer-only ranking formula (Blueprint 5.3).
fn score_entry(
    e: &CatalogEntry,
    q: &str,
    q_grams: &[u32],
    device_tier: u8,
    is_installed: bool,
) -> i32 {
    let id_norm = normalize(&e.id);
    let name_norm = normalize(&e.name);

    let exact_id = i32::from(id_norm == q);
    let name_prefix = i32::from(name_norm.starts_with(q) || id_norm.starts_with(q));
    let kw_exact = i32::from(e.keywords.iter().any(|k| normalize(k) == q));

    let coverage_x1000: i32 = if q_grams.is_empty() {
        1000
    } else {
        let mut matched = 0i32;
        for &g in q_grams {
            if contains_gram(&id_norm, g) || contains_gram(&name_norm, g) {
                matched += 1;
            }
        }
        (matched * 1000) / (q_grams.len() as i32)
    };

    let pop_term = (i32::from(e.popularity) * 10) / 100;
    let tier_fit = i32::from(e.tier_min <= device_tier);
    let heavy_penalty = i32::from(device_tier == 0 && e.ram_active_mb >= 512);

    100 * exact_id
        + 60 * name_prefix
        + 30 * kw_exact
        + (15 * coverage_x1000) / 1000
        + pop_term
        + 8 * i32::from(is_installed)
        + 5 * tier_fit
        - 20 * heavy_penalty
}

fn contains_gram(s: &str, target: u32) -> bool {
    let bytes = s.as_bytes();
    if bytes.len() < 3 {
        return false;
    }
    bytes
        .windows(3)
        .any(|w| pack_trigram(w[0], w[1], w[2]) == target)
}

pub fn normalize(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_space = false;
    for ch in s.trim().chars() {
        let lower = ch.to_ascii_lowercase();
        if lower.is_ascii_whitespace() {
            if !prev_space {
                out.push(' ');
                prev_space = true;
            }
        } else if lower.is_ascii_alphanumeric() || lower == '-' || lower == '_' || lower == '.' {
            out.push(lower);
            prev_space = false;
        }
    }
    out
}

fn pack_trigram(a: u8, b: u8, c: u8) -> u32 {
    ((a as u32) << 16) | ((b as u32) << 8) | (c as u32)
}

fn extract_trigrams(norm: &str) -> Vec<u32> {
    let bytes = norm.as_bytes();
    if bytes.len() < 3 {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(bytes.len() - 2);
    for w in bytes.windows(3) {
        out.push(pack_trigram(w[0], w[1], w[2]));
    }
    out
}

fn intersect_sorted(lists: &[&[u32]]) -> Vec<u32> {
    let mut current: Vec<u32> = lists[0].to_vec();
    for other in &lists[1..] {
        let mut next = Vec::with_capacity(current.len().min(other.len()));
        let mut i = 0;
        let mut j = 0;
        while i < current.len() && j < other.len() {
            match current[i].cmp(&other[j]) {
                std::cmp::Ordering::Equal => {
                    next.push(current[i]);
                    i += 1;
                    j += 1;
                }
                std::cmp::Ordering::Less => i += 1,
                std::cmp::Ordering::Greater => j += 1,
            }
        }
        current = next;
        if current.is_empty() {
            break;
        }
    }
    current
}

fn levenshtein_le2(a: &str, b: &str) -> bool {
    let ab = a.as_bytes();
    let bb = b.as_bytes();
    if ab.len().abs_diff(bb.len()) > 2 {
        return false;
    }
    let m = bb.len();
    let mut prev: Vec<usize> = (0..=m).collect();
    let mut curr = vec![0usize; m + 1];

    for (i, &ca) in ab.iter().enumerate() {
        curr[0] = i + 1;
        let mut row_min = curr[0];
        for (j, &cb) in bb.iter().enumerate() {
            let cost = usize::from(ca != cb);
            let del = prev[j + 1] + 1;
            let ins = curr[j] + 1;
            let sub = prev[j] + cost;
            curr[j + 1] = del.min(ins).min(sub);
            row_min = row_min.min(curr[j + 1]);
        }
        if row_min > 2 {
            return false;
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[m] <= 2
}

fn fx_hash(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in s.as_bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}
