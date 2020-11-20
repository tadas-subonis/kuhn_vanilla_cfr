#[macro_use]
extern crate ndarray;

use itertools::Itertools;
use std::collections::HashMap;
use ndarray::prelude::*;

const _N_ACTIONS: i16 = 2;
const _N_CARDS: i16 = 3;


fn is_chance_node(history: &str) -> bool {
    history == ""
}

fn is_terminal(history: &str) -> bool {
    match history {
        "rrcc" => true,
        "rrcbc" => true,
        "rrcbb" => true,
        "rrbc" => true,
        "rrbb" => true,
        _ => false,
    }
}

fn card_str(card: i16) -> &'static str {
    if card == 0 {
        return "J";
    } else if card == 1 {
        return "Q";
    }
    return "K";
}

#[derive(Debug, Clone)]
struct InformationSet {
    key: String,
    regret_sum: Array1<f64>,
    strategy_sum: Array1<f64>,
    strategy: Array1<f64>,
    reach_pr: f64,
    reach_pr_sum: f64,
}

impl InformationSet {
    fn new(key: String) -> Self {
        InformationSet {
            key,
            regret_sum: Array1::<f64>::zeros(Ix1(_N_ACTIONS as usize)),
            strategy_sum: Array1::<f64>::zeros(Ix1(_N_ACTIONS as usize)),
            strategy: Array1::<f64>::ones(Ix1(_N_ACTIONS as usize)) / (_N_ACTIONS as f64),
            reach_pr: 0.0,
            reach_pr_sum: 0.0,
        }
    }

    fn next_strategy(&mut self) {
        let reach_pr = self.reach_pr;
        let strategy = self.strategy.clone();

        self.strategy_sum = self.strategy_sum.clone() + (strategy * reach_pr);
        self.strategy = self.calc_strategy();
        self.reach_pr_sum += self.reach_pr;
        self.reach_pr = 0.0;
    }

    fn calc_strategy(&self) -> Array1::<f64> {
        let mut strategy = InformationSet::make_positive(self.regret_sum.clone());
        let total = strategy.sum();

        if total > 0.0 {
            strategy = strategy / total;
        } else {
            let n = _N_ACTIONS;
            strategy = Array1::<f64>::ones(Ix1(n as usize)) / (n as f64);
        }

        return strategy;
    }

    fn get_average_strategy(&self) -> Array1::<f64> {
        let mut strategy = self.strategy_sum.clone() / self.reach_pr_sum;
        // strategy = np.where(strategy < 0.001, 0, strategy)
        strategy = strategy.mapv(|a| if a < 0.001 { 0.0 } else { a });
        // println!("Strategy {}", self.strategy_sum);

        let total = strategy.sum();
        strategy = strategy / total;

        return strategy;
    }

    fn make_positive(x: Array1::<f64>) -> Array1::<f64> {
        x.mapv(|a| if a > 0.0 { a } else { 0.0 })
    }
}

// fn get_info_set<'a>(i_map: &'a mut HashMap<String, InformationSet>, card: i16, history: &str) -> &'a mut InformationSet {
//     let key = format!("{} ", card) + history;
//
//     if !i_map.contains_key(&key) {
//         let info_set = InformationSet::new(key.clone());
//         i_map.insert(key.clone(), info_set);
//         // let value = i_map.get_mut(&key).unwrap();
//         // let value = HashMap::<&'a String, InformationSet>::get_mut(&'a mut i_map, &key).unwrap();
//         // return value;
//     }
//
//     return i_map.get_mut(&key).unwrap();
// }

fn terminal_util(history: &str, card_1: i16, card_2: i16) -> f64 {
    let n = history.len();

    let card_player = if n % 2 == 0 { card_1 } else { card_2 };
    let card_opponent = if n % 2 == 0 { card_2 } else { card_1 };

    match history {
        "rrcbc" => 1.0,
        "rrbc" => 1.0,
        "rrcc" => if card_player > card_opponent { 1.0 } else { -1.0 },
        "rrcbb" => if card_player > card_opponent { 2.0 } else { -2.0 },
        "rrbb" => if card_player > card_opponent { 2.0 } else { -2.0 },
        _ => panic!("Illegal line")
    }
}

fn chance_util(mut i_map: HashMap<String, InformationSet>) -> (f64, HashMap<String, InformationSet>) {
    let mut expected_value = 0.0;
    let n_possibilities = 6;

    for i in 0.._N_CARDS {
        for j in 0.._N_CARDS {
            if i != j {
                let (util, n_map) = cfr(i_map, "rr", i, j, 1.0, 1.0, 1.0 / (n_possibilities as f64));
                expected_value = util;
                i_map = n_map;
            }
        }
    }

    return (expected_value / (n_possibilities as f64), i_map);
}

fn cfr(mut i_map: HashMap<String, InformationSet>, history: &str, card_1: i16, card_2: i16, pr_1: f64, pr_2: f64, pr_c: f64) -> (f64, HashMap<String, InformationSet>) {
    if is_chance_node(history) {
        return chance_util(i_map);
    }
    if is_terminal(history) {
        return (terminal_util(history, card_1, card_2), i_map);
    }

    let n = history.len();
    let is_player_1 = n % 2 == 0;
    // let info_set = get_info_set(i_map, if is_player_1 { card_1 } else { card_2 }, history);

    let card = if is_player_1 { card_1 } else { card_2 };
    let key = format!("{} ", card_str(card)) + history;
    let info_set = i_map.entry(key.clone()).or_insert_with(|| InformationSet::new(key.clone()));
    info_set.reach_pr += if is_player_1 { pr_1 } else { pr_2 };
    let strategy = info_set.strategy.clone();

    // Counterfactual utility per action.
    let mut action_utils = Array::zeros(Ix1(_N_ACTIONS as usize));
    for (i, action) in vec!["c", "b"].iter().enumerate() {
        let next_history = format!("{}{}", history, action);
        let pr_1_s = if is_player_1 { pr_1 * strategy[i] } else { pr_1 };
        let pr_2_s = if is_player_1 { pr_2 } else { pr_2 * strategy[i] };

        let (util, n_map) = cfr(i_map, next_history.as_str(), card_1, card_2, pr_1_s, pr_2_s, pr_c);
        action_utils[i] = -1.0 * util;
        i_map = n_map;
    }

    // Utility of information set.
    let util: f64 = (action_utils.clone() * strategy).scalar_sum();
    let regrets: Array1<f64> = action_utils.clone() - util;
    // println!("Regrests {}", regrets);

    let pr = if is_player_1 { pr_2 } else { pr_1 };

    let info_set = i_map.get_mut(&key).unwrap();
    info_set.regret_sum = info_set.regret_sum.clone() + regrets * pr * pr_c;
    // println!("info_set.regret_sum {}", info_set.regret_sum);

    // println!("Map size {}", i_map.len());

    return (util, i_map);
}

impl<'a> std::fmt::Display for InformationSet {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let strategies = self.get_average_strategy().iter()
            .map(|e| format!("{:.2}", e))
            .join(" ");

        write!(f, "{} {}", self.key, strategies)
    }
}

fn display_results(ev: f64, i_map: &HashMap<String, InformationSet>) {
    println!("player 1 expected value: {}", ev);
    println!("player 2 expected value: {}", -ev);
    println!();

    println!("player 1 strategies:");
    for (_, info_set) in i_map.iter()
        .filter(|(a, _)| a.len() % 2 == 0)
        .sorted_by_key(|(a, _)| a.clone()) {
        println!("{}", info_set);
    }

    println!("player 2 strategies:");
    for (_, info_set) in i_map.iter()
        .filter(|(a, _)| a.len() % 2 == 1)
        .sorted_by_key(|(a, _)| a.clone()) {
        println!("{}", info_set);
    }
}

fn main() {
    let mut i_map: HashMap<String, InformationSet> = HashMap::new();
    let n_iterations = 10000;
    let mut expected_game_value = 0.0;

    for _ in 0..n_iterations {
        let (util, n_map) = cfr(i_map, "", -1, -1, 1.0, 1.0, 1.0);

        expected_game_value += util;
        i_map = n_map;

        for (_, v) in i_map.iter_mut() {
            v.next_strategy();
            // println!("Strategy v {}", v.strategy);
        }
    }

    expected_game_value /= n_iterations as f64;

    display_results(expected_game_value, &i_map)
}
