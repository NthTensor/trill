use core::f32;
use std::collections::HashMap;
use std::hash::BuildHasherDefault;
use std::hash::Hash;
use std::hash::Hasher;

use bevy_mod_props::Props;
use bevy_mod_props::Value;
use itertools::Itertools;
use rand::rngs::ThreadRng;
use rand::seq::IndexedRandom;
use rand::seq::SliceRandom;
use ustr::Ustr;
use ustr::UstrMap;

use crate::Operation;
use crate::ResponseEngineCompiler;

pub(crate) struct Encoder {
    next_float: f32,
    encodings: UstrMap<f32>,
}

impl Default for Encoder {
    fn default() -> Encoder {
        Encoder {
            next_float: f32::MIN,
            encodings: UstrMap::default(),
        }
    }
}

impl Encoder {
    pub fn encode_ustr(&mut self, ustr: Ustr) -> f32 {
        let encoding = self.encodings.entry(ustr).or_insert_with(|| {
            let encoding = self.next_float;
            self.next_float = self.next_float.next_up();
            encoding
        });
        *encoding
    }

    pub fn encode(&mut self, value: Value) -> f32 {
        match value {
            Value::Bool(false) => 0.0,
            Value::Bool(true) => 1.0,
            Value::Num(num) => num,
            Value::Str(ustr) => self.encode_ustr(ustr),
        }
    }
}

#[derive(Debug)]
struct Query {
    scanners: Vec<Scanner>,
}

impl Query {
    fn build<'q, I>(props_list: I, encoder: &mut Encoder) -> Query
    where
        I: IntoIterator<Item = &'q Props>,
    {
        let scanners = props_list
            .into_iter()
            .map(|s| {
                let items = s
                    .iter()
                    .map(|(name, value)| (*name, encoder.encode(*value)))
                    .collect::<Vec<_>>();
                Scanner::new(items)
            })
            .collect();
        Query { scanners }
    }

    fn scan_to(&mut self, var_name: Ustr) -> Option<f32> {
        self.scanners.iter_mut().find_map(|s| s.scan_to(var_name))
    }

    fn reset(&mut self) {
        self.scanners.iter_mut().for_each(Scanner::reset)
    }
}

#[derive(Debug)]
struct Scanner {
    items: Vec<(Ustr, f32)>,
    cursor: usize,
}

impl Scanner {
    fn new(items: Vec<(Ustr, f32)>) -> Scanner {
        Scanner { items, cursor: 0 }
    }

    // Looks up the value of a key. Repeated calls should use keys of increasing order.
    fn scan_to(&mut self, variable: Ustr) -> Option<f32> {
        let search_result = self.items[self.cursor..]
            .iter()
            .position(|(var, _)| var.ge(&variable));
        match search_result {
            Some(i) => {
                self.cursor += i;
                let (var, value) = self.items[self.cursor];
                if var.eq(&variable) { Some(value) } else { None }
            }
            None => {
                self.cursor = self.items.len();
                None
            }
        }
    }

    fn reset(&mut self) {
        self.cursor = 0;
    }
}

pub struct ResponseEngine {
    pub(crate) criteria: Vec<EngineCriterion>,
    pub(crate) rules: RulePartitions, // rules grouped into partitions, then sorted by importance
    pub(crate) response_groups: Vec<EngineResponseGroup>,
    // Converts interned strings to floating point values
    pub(crate) encoder: Encoder,
}

impl ResponseEngine {
    pub fn build() -> ResponseEngineCompiler {
        ResponseEngineCompiler::new()
    }

    pub fn find_best_response<'q>(
        &mut self,
        request_props: &'q Props,
        mut charicter_props: &'q mut Props,
        mut world_props: &'q mut Props,
        rng: &mut ThreadRng,
    ) -> Option<&UstrMap<String>> {
        let query = Query::build(
            [request_props, charicter_props, world_props],
            &mut self.encoder,
        );

        let mut response = None;
        if let Some((key, index)) = self.find_best_matching_rule(query, rng) {
            let rule = self.rules.get_rule_mut(&key, index);

            for (var, (global, op)) in &rule.instructions {
                let props = if *global {
                    &mut world_props
                } else {
                    &mut charicter_props
                };
                let value = props.get_value(*var);
                match (value, *op) {
                    (Some(Value::Bool(value)), Operation::BoolToggle) => props.set(*var, !value),
                    (Some(Value::Num(value)), Operation::NumAdd(num)) => {
                        props.set(*var, value + num)
                    }
                    (_, Operation::BoolSet(bool)) => props.set(*var, bool),
                    (_, Operation::BoolToggle) => props.set(*var, true),
                    (_, Operation::NumSet(num)) => props.set(*var, num),
                    (_, Operation::NumAdd(num)) => props.set(*var, num),
                    (_, Operation::StrSet(ustr)) => props.set(*var, self.encoder.encode_ustr(ustr)),
                }
            }

            // Query for a response from each response group, in a random order
            let mut group_indicies = rule.response_groups.clone();
            group_indicies.shuffle(rng);
            for group_index in group_indicies {
                let group = &mut self.response_groups[group_index];
                if let Some(response_index) = group.dispatcher.next(rng) {
                    response = Some((group_index, response_index));

                    if group.dispatcher.disable_rule() {
                        rule.enabled = false;
                    }

                    break;
                }
            }
        }
        response.map(|(g, i)| &self.response_groups[g].responses[i])
    }

    fn find_best_matching_rule(
        &mut self,
        mut query: Query,
        rng: &mut ThreadRng,
    ) -> Option<(PartitionKey, usize)> {
        let mut best_score = 0.0;
        let mut best_rules = Vec::new();

        for key in self.rules.get_partition_keys_for_query(&mut query) {
            let partition = self.rules.get_partition(&key);
            for (i, rule) in partition.iter().enumerate() {
                // First, check the score. Rules are stored by decreasing score,
                // so once we encounter a rule that's worse than the best thing
                // we've found so far, we can stop.
                if rule.score < best_score {
                    break;
                }
                // If it scores better or equal to our current best, check to
                // see if the criteria match.
                if self.match_rule_criteria(&mut query, rule) {
                    if rule.score > best_score {
                        // If the criteria are a match and it scores better, throw out what we have.
                        best_score = rule.score;
                        best_rules.clear();
                        best_rules.push((key, i));
                    } else {
                        // Otherwise the score must be equal, and we include it in the list.
                        best_rules.push((key, i));
                    }
                }
            }
        }

        // Choose a random rule from the list of matches
        best_rules.choose(rng).cloned()
    }

    fn match_rule_criteria(&self, query: &mut Query, rule: &EngineRule) -> bool {
        query.reset();
        for criterion_index in &rule.criteria {
            let criterion = &self.criteria[*criterion_index];
            if let Some(value) = query.scan_to(criterion.variable) {
                if criterion.min <= value && value <= criterion.max {
                } else {
                    return false;
                }
            } else {
                return false;
            }
        }
        true
    }
}

#[derive(Debug)]
pub(crate) struct EngineRule {
    pub criteria: Vec<usize>, // Sorted by variable name (increasing)
    pub response_groups: Vec<usize>,
    pub instructions: UstrMap<(bool, Operation)>,
    pub score: f32,
    pub enabled: bool,
}

#[derive(Debug)]
pub(crate) struct EngineCriterion {
    pub variable: Ustr,
    pub min: f32,
    pub max: f32,
}

#[derive(Debug)]
pub(crate) struct EngineResponseGroup {
    pub dispatcher: ResponseDispatcher,
    pub responses: Vec<UstrMap<String>>,
}

#[derive(Debug)]
pub enum ResponseDispatcher {
    Shuffle {
        weights: Vec<f32>,
        candidates: Vec<usize>,
    },
    Random {
        weights: Vec<f32>,
    },
    Deplete {
        weights: Vec<f32>,
        candidates: Vec<usize>,
    },
    Loop {
        len: usize,
        index: usize,
    },
    List {
        len: usize,
        index: usize,
    },
}

impl ResponseDispatcher {
    fn next(&mut self, rng: &mut ThreadRng) -> Option<usize> {
        match self {
            ResponseDispatcher::Shuffle {
                weights,
                candidates,
            } => {
                if weights.len() == 1 {
                    return Some(0);
                }
                let candidate_indicies: Vec<_> = (0..candidates.len()).collect();
                let i = candidate_indicies
                    .choose_weighted(rng, |i| weights[candidates[*i]])
                    .ok()?;
                let i = candidates.remove(*i);
                if candidates.len() == 0 {
                    *candidates = (0..weights.len()).collect();
                    let _ = candidates.remove(i);
                }
                Some(i)
            }
            ResponseDispatcher::Random { weights } => {
                if weights.len() == 1 {
                    return Some(0);
                }
                let candidates: Vec<_> = (0..weights.len()).collect();
                candidates
                    .choose_weighted(rng, |i| weights[*i])
                    .ok()
                    .copied()
            }
            ResponseDispatcher::Deplete {
                weights,
                candidates,
            } => {
                let candidate_indicies: Vec<_> = (0..candidates.len()).collect();
                let i = candidate_indicies
                    .choose_weighted(rng, |i| weights[candidates[*i]])
                    .ok()?;
                let i = candidates.remove(*i);
                Some(i)
            }
            ResponseDispatcher::Loop { len, index } => {
                let i = *index;
                *index = (*index + 1) % *len;
                Some(i)
            }
            ResponseDispatcher::List { len, index } => {
                if *index < *len {
                    let i = *index;
                    *index += 1;
                    Some(i)
                } else {
                    None
                }
            }
        }
    }

    fn disable_rule(&self) -> bool {
        match self {
            // These dispatchers will never run out of items
            ResponseDispatcher::Shuffle { .. }
            | ResponseDispatcher::Loop { .. }
            | ResponseDispatcher::Random { .. } => false,
            // Disable deplete when the candidate list is empty
            ResponseDispatcher::Deplete { candidates, .. } => candidates.is_empty(),
            // Diable list when we reach the end of the list
            ResponseDispatcher::List { len, index } => *len == *index,
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub(crate) struct PartitionKey(u64);

#[derive(Debug)]
pub(crate) struct RulePartitions {
    pub vars: Vec<Ustr>, // Sorted by variable name (increasing)
    pub partitions: HashMap<PartitionKey, Vec<EngineRule>, BuildHasherDefault<IdentityHasher>>,
}

impl RulePartitions {
    // Returns the keys to all partitions that might contain relevant rules
    fn get_partition_keys_for_query(&self, query: &mut Query) -> Vec<PartitionKey> {
        query.reset();
        let mut assignments = Vec::with_capacity(self.vars.len());
        for var in &self.vars {
            if let Some(value) = query.scan_to(*var) {
                assignments.push((*var, value));
            }
        }

        assignments
            .into_iter()
            .powerset()
            .map(|assignments| self.get_partition_key_for_assignments(&assignments))
            .collect()
    }

    // Returns the key for this set of variable assignments
    pub fn get_partition_key_for_assignments(&self, assignments: &[(Ustr, f32)]) -> PartitionKey {
        use rapidhash::fast::RapidHasher;

        // Do manual hashing here because it's an array and it contains floats
        let mut hasher = RapidHasher::default_const();
        for (variable, value) in assignments {
            variable.hash(&mut hasher);
            value.to_bits().hash(&mut hasher);
        }
        PartitionKey(hasher.finish())
    }

    // Accesses the partition with the given key
    fn get_partition(&self, key: &PartitionKey) -> &[EngineRule] {
        self.partitions.get(key).map(Vec::as_slice).unwrap_or(&[])
    }

    fn get_rule_mut(&mut self, key: &PartitionKey, rule_index: usize) -> &mut EngineRule {
        &mut self.partitions.get_mut(key).unwrap()[rule_index]
    }
}

#[doc(hidden)]
#[derive(Default)]
pub(crate) struct IdentityHasher {
    hash: u64,
}

impl Hasher for IdentityHasher {
    #[inline]
    fn write(&mut self, _bytes: &[u8]) {
        panic!("Use `write_u64` instead.");
    }

    #[inline]
    fn finish(&self) -> u64 {
        self.hash
    }

    #[inline]
    fn write_u64(&mut self, i: u64) {
        self.hash = i;
    }
}
