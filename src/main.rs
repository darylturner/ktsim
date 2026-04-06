use clap::{Args, Parser, Subcommand, ValueEnum};
use rand::Rng;
use std::collections::HashMap;

const WIDTH: usize = 72;
const BAR_WIDTH_HITS: usize = 28;
const BAR_WIDTH_FULL: usize = 20;

#[derive(Parser)]
#[command(
    name = "ktsim",
    about = "Kill Team Monte Carlo simulations",
    before_help = r#"
          .                                                      .
        .n                   .                 .                  n.
  .   .dP                  dP                   9b                 9b.    .
 4    qXb         .       dX                     Xb       .        dXp     t
dX.    9Xb      .dXb    __                         __    dXb.     dXP     .Xb
9XXb._       _.dXXXXb dXXXXbo.                 .odXXXXb dXXXXb._       _.dXXP
 9XXXXXXXXXXXXXXXXXXXVXXXXXXXXOo.           .oOXXXXXXXXVXXXXXXXXXXXXXXXXXXXP
  `9XXXXXXXXXXXXXXXXXXXXX'~   ~`OOO8b   d8OOO'~   ~`XXXXXXXXXXXXXXXXXXXXXP'
    `9XXXXXXXXXXXP' `9XX'    KT    `98v8P'   SIM    `XXP' `9XXXXXXXXXXXP'
        ~~~~~~~       9X.          .db|db.          .XP       ~~~~~~~
                        )b.  .dbo.dP'`v'`9b.odb.  .dX(
                      ,dXXXXXXXXXXXb     dXXXXXXXXXXXb.
                     dXXXXXXXXXXXP'   .   `9XXXXXXXXXXXb
                    dXXXXXXXXXXXXb   d|b   dXXXXXXXXXXXXb
                    9XXb'   `XXXXXb.dX|Xb.dXXXXX'   `dXXP
                     `'      9XXXXXX(   )XXXXXXP      `'
                              XXXX X.`v'.X XXXX
                              XP^X'`b   d'`X^XX
                              X. 9  `   '  P )X
                              `b  `       '  d'
                               `             '
"#
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Simulate an attack roll
    Attack(AttackArgs),
    /// Simulate a defence roll
    Defence(DefenceArgs),
}

#[derive(Args)]
#[command(disable_help_flag = true)]
struct AttackArgs {
    /// Number of attack dice
    #[arg(short = 'a', long, default_value_t = 4)]
    attacks: usize,

    /// Hit threshold — roll this value or higher to hit (2–6)
    #[arg(short = 'h', long, default_value_t = 3, value_parser = clap::value_parser!(u8).range(2..=6))]
    hit: u8,

    /// Reroll ability
    #[arg(short, long, value_enum, default_value_t = Reroll::None)]
    reroll: Reroll,

    /// Lethal threshold — roll this value or higher for a critical hit (default: 6)
    #[arg(short, long, default_value_t = 6, value_parser = clap::value_parser!(u8).range(2..=6))]
    lethal: u8,

    /// Punishing: if at least one critical is rolled, convert one miss to a normal hit
    #[arg(long, default_value_t = false)]
    punishing: bool,

    /// Rending: if at least one critical is rolled, convert one normal hit to a critical
    #[arg(long, default_value_t = false)]
    rending: bool,

    /// Severe: if no criticals are rolled, convert one normal hit to a critical (cannot trigger punishing or rending)
    #[arg(long, default_value_t = false)]
    severe: bool,

    /// Accurate: number of dice automatically retained as normal hits (unaffected by special rules)
    #[arg(long, default_value_t = 0)]
    accurate: usize,

    /// Number of simulations
    #[arg(short, long, default_value_t = 10_000)]
    sims: usize,

    /// Output format
    #[arg(short, long, value_enum, default_value_t = Output::Hits)]
    output: Output,

    #[arg(long, action = clap::ArgAction::Help)]
    help: Option<bool>,
}

#[derive(Args)]
struct DefenceArgs {
    /// Number of save dice
    #[arg(long, default_value_t = 3)]
    saves: usize,

    /// Save characteristic — roll this value or higher to save (2–6)
    #[arg(short = 'S', long, default_value_t = 4, value_parser = clap::value_parser!(u8).range(2..=6))]
    save: u8,

    /// Retained normal saves — reduces the rolled dice pool
    #[arg(long, default_value_t = 0)]
    retained_normals: usize,

    /// Retained critical saves — reduces the rolled dice pool
    #[arg(long, default_value_t = 0)]
    retained_crits: usize,

    /// Reroll ability
    #[arg(short, long, value_enum, default_value_t = Reroll::None)]
    reroll: Reroll,

    /// Number of simulations
    #[arg(short, long, default_value_t = 10_000)]
    sims: usize,

    /// Output format
    #[arg(short, long, value_enum, default_value_t = Output::Hits)]
    output: Output,
}

#[derive(Clone, ValueEnum)]
enum Output {
    /// Hits distribution only
    Hits,
    /// Full breakdown of crits, normals, and misses per combination
    Full,
}

#[derive(Clone, ValueEnum)]
enum Reroll {
    /// No rerolls
    None,
    /// Reroll 1 miss
    Balanced,
    /// Reroll the largest group of misses (most frequent missed value)
    Ceaseless,
    /// Reroll all misses
    Relentless,
}

struct Retain {
    normals: usize,
    crits: usize,
}

struct WeaponRules {
    punishing: bool,
    rending: bool,
    severe: bool,
    lethal: u8,
}

#[derive(Debug, PartialEq)]
struct SimResult {
    misses: usize,
    normals: usize,
    crits: usize,
}

impl SimResult {
    fn hits(&self) -> usize {
        self.normals + self.crits
    }
}

fn roll_d6(rng: &mut impl Rng) -> u8 {
    rng.gen_range(1..=6)
}

fn apply_rerolls(rolls: &mut [u8], threshold: u8, reroll: &Reroll, rng: &mut impl Rng) {
    match reroll {
        Reroll::None => {}

        Reroll::Balanced => {
            // reroll the first missed die
            if let Some(pos) = rolls.iter().position(|&v| v < threshold) {
                rolls[pos] = roll_d6(rng);
            }
        }

        Reroll::Ceaseless => {
            // reroll the largest group of misses (most frequent missed value)
            let mut freq: HashMap<u8, usize> = HashMap::new();
            for &v in rolls.iter() {
                if v < threshold {
                    *freq.entry(v).or_insert(0) += 1;
                }
            }
            let target = freq
                .iter()
                .max_by_key(|&(_, &cnt)| cnt)
                .map(|(&val, _)| val);
            if let Some(target) = target {
                for v in rolls.iter_mut() {
                    if *v == target {
                        *v = roll_d6(rng);
                    }
                }
            }
        }

        Reroll::Relentless => {
            // reroll every miss
            for v in rolls.iter_mut() {
                if *v < threshold {
                    *v = roll_d6(rng);
                }
            }
        }
    }
}

fn classify_rolls(rolls: &[u8], threshold: u8, weapon_rules: Option<&WeaponRules>) -> SimResult {
    let lethal = weapon_rules.map_or(6, |r| r.lethal);
    let mut misses = 0;
    let mut normals = 0;
    let mut crits = 0;

    for &v in rolls.iter() {
        if v < threshold {
            misses += 1;
        } else if v >= lethal {
            crits += 1;
        } else {
            normals += 1;
        }
    }

    if let Some(rules) = weapon_rules {
        // punishing: crit converts a miss to a normal
        if rules.punishing && crits >= 1 && misses >= 1 {
            misses -= 1;
            normals += 1;
        }
        // rending: crit converts a normal to a crit
        if rules.rending && crits >= 1 && normals >= 1 {
            normals -= 1;
            crits += 1;
        }
        // severe: no crits converts a normal to a crit (cannot trigger punishing or rending)
        if rules.severe && crits == 0 && normals >= 1 {
            normals -= 1;
            crits += 1;
        }
    }

    SimResult { misses, normals, crits }
}

// simresult struct is 24 bytes so expect 24xsim bytes for memory usage
// 10m simulations seems on the high side here so 240mB for the vec<simresult>
fn simulate_rolls(
    attacks: usize,
    threshold: u8,
    reroll: &Reroll,
    weapon_rules: &WeaponRules,
    retain: &Retain,
    sims: usize,
    rng: &mut impl Rng,
) -> Vec<SimResult> {
    let dice = attacks.saturating_sub(retain.normals + retain.crits);
    (0..sims)
        .map(|_| {
            let mut rolls: Vec<u8> = (0..dice).map(|_| roll_d6(rng)).collect();
            apply_rerolls(&mut rolls, threshold, reroll, rng);
            let mut result = classify_rolls(&rolls, threshold, Some(weapon_rules));
            result.normals += retain.normals;
            result.crits += retain.crits;
            result
        })
        .collect()
}

fn simulate_defence(
    saves: usize,
    threshold: u8,
    reroll: &Reroll,
    retain: &Retain,
    sims: usize,
    rng: &mut impl Rng,
) -> Vec<SimResult> {
    let dice = saves.saturating_sub(retain.normals + retain.crits);
    (0..sims)
        .map(|_| {
            let mut rolls: Vec<u8> = (0..dice).map(|_| roll_d6(rng)).collect();
            apply_rerolls(&mut rolls, threshold, reroll, rng);
            let mut result = classify_rolls(&rolls, threshold, None);
            result.normals += retain.normals;
            result.crits += retain.crits;
            result
        })
        .collect()
}

fn reroll_label(reroll: &Reroll) -> &'static str {
    match reroll {
        Reroll::None => "None",
        Reroll::Balanced => "Balanced (reroll 1 miss)",
        Reroll::Ceaseless => "Ceaseless (reroll largest group of misses)",
        Reroll::Relentless => "Relentless (reroll all misses)",
    }
}

fn print_stats(results: &[SimResult], total_dice: usize, hit_label: &str, output: &Output) {
    let total = results.len() as f64;

    let mean_crits = results.iter().map(|r| r.crits).sum::<usize>() as f64 / total;
    let mean_normals = results.iter().map(|r| r.normals).sum::<usize>() as f64 / total;
    let mean_misses = results.iter().map(|r| r.misses).sum::<usize>() as f64 / total;
    let mean_hits = results.iter().map(|r| r.hits()).sum::<usize>() as f64 / total;

    let variance = results
        .iter()
        .map(|r| (r.hits() as f64 - mean_hits).powi(2))
        .sum::<f64>()
        / total;
    let std_dev = variance.sqrt();

    let mut hit_counts: HashMap<usize, usize> = HashMap::new();
    for r in results {
        *hit_counts.entry(r.hits()).or_insert(0) += 1;
    }

    let mut sorted_hits: Vec<usize> = results.iter().map(|r| r.hits()).collect();
    sorted_hits.sort_unstable();
    let median = sorted_hits[results.len() / 2];

    println!();
    println!("  Mean {}  : {:.3}  ({:.3} normal + {:.3} crit)", hit_label, mean_hits, mean_normals, mean_crits);
    println!("  Mean misses : {:.3}", mean_misses);
    println!("  Median {}  : {}", hit_label, median);
    println!("  Std dev     : {:.3}", std_dev);
    println!("  Range       : {} – {}", sorted_hits.first().unwrap(), sorted_hits.last().unwrap());

    match output {
        Output::Hits => print_hits_table(total_dice, &hit_counts, total, mean_hits, hit_label),
        Output::Full => print_breakdown_table(results, total_dice, &hit_counts, total),
    }
}

fn print_attack_results(
    results: &[SimResult],
    args: &AttackArgs,
    weapon_rules: &WeaponRules,
) {
    println!();
    println!("{}", "=".repeat(WIDTH));
    println!("  Kill Team Dice Simulator — Attack");
    println!("{}", "=".repeat(WIDTH));
    println!("  Attacks     : {}", args.attacks);
    println!("  Hit         : {}+", args.hit);
    println!("  Lethal      : {}+", weapon_rules.lethal);
    println!("  Accurate    : {}", args.accurate);
    println!("  Punishing   : {}", if weapon_rules.punishing { "Yes" } else { "No" });
    println!("  Rending     : {}", if weapon_rules.rending { "Yes" } else { "No" });
    println!("  Severe      : {}", if weapon_rules.severe { "Yes" } else { "No" });
    println!("  Rerolls     : {}", reroll_label(&args.reroll));
    println!("  Simulations : {}", format_num(args.sims));
    println!("{}", "=".repeat(WIDTH));

    print_stats(results, args.attacks, "hits", &args.output);
}

fn print_defence_results(
    results: &[SimResult],
    args: &DefenceArgs,
) {
    println!();
    println!("{}", "=".repeat(WIDTH));
    println!("  Kill Team Dice Simulator — Defence");
    println!("{}", "=".repeat(WIDTH));
    println!("  Save Dice   : {}", args.saves);
    println!("  Save        : {}+", args.save);
    println!("  Retained    : {} normal, {} critical", args.retained_normals, args.retained_crits);
    println!("  Rerolls     : {}", reroll_label(&args.reroll));
    println!("  Simulations : {}", format_num(args.sims));
    println!("{}", "=".repeat(WIDTH));

    print_stats(results, args.saves, "saves", &args.output);
}

fn make_bar(count: usize, max_count: usize, width: usize) -> String {
    "█".repeat((count * width) / max_count.max(1))
}

fn print_hits_table(
    dice: usize,
    hit_counts: &HashMap<usize, usize>,
    total: f64,
    mean_hits: f64,
    label: &str,
) {
    let max_count = *hit_counts.values().max().unwrap_or(&1);

    println!();
    println!("{}", "─".repeat(WIDTH));
    println!(
        "  {:<6} {:<9} {:>6}   {:>6}   {}",
        label, "Count", "Prob", "≥ Prob", "Distribution"
    );
    println!("{}", "─".repeat(WIDTH));

    for s in 0..=dice {
        let count = hit_counts.get(&s).copied().unwrap_or(0);
        let prob = count as f64 / total;
        let cum_prob = (s..=dice)
            .map(|k| hit_counts.get(&k).copied().unwrap_or(0))
            .sum::<usize>() as f64
            / total;
        let bar = make_bar(count, max_count, BAR_WIDTH_HITS);
        let marker = if s == mean_hits.round() as usize {
            " ◄ mean"
        } else {
            ""
        };
        println!(
            "  {:<6} {:<9} {:>5.1}%   {:>5.1}%   {}{}",
            s, count, prob * 100.0, cum_prob * 100.0, bar, marker
        );
    }

    println!("{}", "─".repeat(WIDTH));
    println!();
}

fn print_breakdown_table(
    results: &[SimResult],
    dice: usize,
    hit_counts: &HashMap<usize, usize>,
    total: f64,
) {
    let mut combo_counts: HashMap<(usize, usize), usize> = HashMap::new();
    for r in results {
        *combo_counts.entry((r.crits, r.normals)).or_insert(0) += 1;
    }

    // sort: hits ascending, then crits ascending within same hit total
    let mut combos: Vec<((usize, usize), usize)> = combo_counts.into_iter().collect();
    combos.sort_by(|&((crit_a, norm_a), _), &((crit_b, norm_b), _)| {
        (crit_a + norm_a).cmp(&(crit_b + norm_b))
            .then(crit_a.cmp(&crit_b))
    });

    let max_count = combos.iter().map(|&(_, c)| c).max().unwrap_or(1);

    println!();
    println!("{}", "─".repeat(WIDTH));
    println!(
        "  {:<5} {:<6} {:<8} {:<8} {:<9} {:>6}   {:>6}   {}",
        "Hits", "Crits", "Normals", "Misses", "Count", "Prob", "≥ Prob", "Distribution"
    );
    println!("{}", "─".repeat(WIDTH));

    for &((crits, normals), count) in combos.iter() {
        let hits = crits + normals;
        let misses = dice - hits;
        let prob = count as f64 / total;
        let cum_prob = (hits..=dice)
            .map(|k| hit_counts.get(&k).copied().unwrap_or(0))
            .sum::<usize>() as f64
            / total;
        let bar = make_bar(count, max_count, BAR_WIDTH_FULL);

        println!(
            "  {:<5} {:<6} {:<8} {:<8} {:<9} {:>5.1}%   {:>5.1}%   {}",
            hits, crits, normals, misses, count, prob * 100.0, cum_prob * 100.0, bar
        );
    }

    println!("{}", "─".repeat(WIDTH));
    println!();
}

fn format_num(n: usize) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Attack(args) => {
            let weapon_rules = WeaponRules {
                punishing: args.punishing,
                rending: args.rending,
                severe: args.severe,
                lethal: args.lethal,
            };
            let results = simulate_rolls(
                args.attacks,
                args.hit,
                &args.reroll,
                &weapon_rules,
                &Retain { normals: args.accurate, crits: 0 },
                args.sims,
                &mut rand::thread_rng(),
            );
            print_attack_results(&results, &args, &weapon_rules);
        }

        Command::Defence(args) => {
            let results = simulate_defence(
                args.saves,
                args.save,
                &args.reroll,
                &Retain { normals: args.retained_normals, crits: args.retained_crits },
                args.sims,
                &mut rand::thread_rng(),
            );
            print_defence_results(&results, &args);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    // seeded with 8, our expected roll sequence for rerolls
    // [4, 3, 6, 5, 4, 6, 4, 3, 5, 4]
    fn seeded_rng() -> rand::rngs::StdRng {
        rand::rngs::StdRng::seed_from_u64(8)
    }

    // fn print_seeded_roll_sequence() {
    //     use rand::{SeedableRng, Rng};
    //     let mut rng = rand::rngs::StdRng::seed_from_u64(8);
    //     let rolls: Vec<u8> = (0..10).map(|_| rng.gen_range(1u8..=6)).collect();
    //     println!("{:?}", rolls);
    // }

    #[test]
    fn test_reroll_balanced() {
        let mut rolls = vec![1, 1, 3, 4, 5];
        apply_rerolls(&mut rolls, 3, &Reroll::Balanced, &mut seeded_rng());
        assert_eq!(rolls, vec![4, 1, 3, 4, 5]);
    }

    #[test]
    fn test_reroll_ceaseless() {
        let mut rolls = vec![1, 1, 2, 4, 5];
        apply_rerolls(&mut rolls, 3, &Reroll::Ceaseless, &mut seeded_rng());
        assert_eq!(rolls, vec![4, 3, 2, 4, 5]);
    }

    #[test]
    fn test_reroll_relentless() {
        let mut rolls = vec![1, 1, 2, 4, 5];
        apply_rerolls(&mut rolls, 3, &Reroll::Relentless, &mut seeded_rng());
        assert_eq!(rolls, vec![4, 3, 6, 4, 5]);
    }

    #[test]
    fn test_classify_no_special() {
        let rolls = vec![1, 1, 2, 4, 6];
        let special = WeaponRules{punishing: false, rending: false, severe: false, lethal: 6};
        let result = classify_rolls(&rolls, 3, Some(&special));
        assert_eq!(result, SimResult{
            misses: 3,
            normals: 1,
            crits: 1,
        });

        assert!(result.hits() == 2);
    }

    #[test]
    fn test_classify_punishing() {
        let rolls = vec![1, 1, 2, 4, 6];
        let special = WeaponRules{punishing: true, rending: false, severe: false, lethal: 6};
        let result = classify_rolls(&rolls, 3, Some(&special));
        assert_eq!(result, SimResult{
            misses: 2,
            normals: 2,
            crits: 1,
        });

        assert!(result.hits() == 3);
    }

    #[test]
    fn test_classify_rending() {
        let rolls = vec![1, 1, 3, 4, 6];
        let special = WeaponRules{punishing: false, rending: true, severe: false, lethal: 6};
        let result = classify_rolls(&rolls, 3, Some(&special));
        assert_eq!(result, SimResult{
            misses: 2,
            normals: 1,
            crits: 2,
        });

        assert!(result.hits() == 3);
    }

    #[test]
    fn test_classify_lethal() {
        let rolls = vec![1, 1, 3, 5, 6];
        let special = WeaponRules{punishing: false, rending: false, severe: false, lethal: 5};
        let result = classify_rolls(&rolls, 3, Some(&special));
        assert_eq!(result, SimResult{
            misses: 2,
            normals: 1,
            crits: 2,
        });

        assert!(result.hits() == 3);
    }

    #[test]
    fn test_classify_severe() {
        let rolls = vec![1, 1, 3, 5, 5];
        let special = WeaponRules{punishing: false, rending: false, severe: true, lethal: 6};
        let result = classify_rolls(&rolls, 3, Some(&special));
        assert_eq!(result, SimResult{
            misses: 2,
            normals: 2,
            crits: 1,
        });

        assert!(result.hits() == 3);
    }

    #[test]
    fn test_classify_severe_with_critical() {
        let rolls = vec![1, 1, 3, 5, 6];
        let special = WeaponRules{punishing: false, rending: false, severe: true, lethal: 6};
        let result = classify_rolls(&rolls, 3, Some(&special));
        assert_eq!(result, SimResult{
            misses: 2,
            normals: 2,
            crits: 1,
        });

        assert!(result.hits() == 3);
    }

    #[test]
    fn test_classify_severe_rending() {
        let rolls = vec![1, 1, 3, 5, 5];
        let special = WeaponRules{punishing: false, rending: true, severe: true, lethal: 6};
        let result = classify_rolls(&rolls, 3, Some(&special));
        assert_eq!(result, SimResult{
            misses: 2,
            normals: 2,
            crits: 1,
        });

        assert!(result.hits() == 3);
    }

    #[test]
    fn test_classify_severe_punishing() {
        let rolls = vec![1, 1, 3, 5, 5];
        let special = WeaponRules{punishing: true, rending: false, severe: true, lethal: 6};
        let result = classify_rolls(&rolls, 3, Some(&special));
        assert_eq!(result, SimResult{
            misses: 2,
            normals: 2,
            crits: 1,
        });

        assert!(result.hits() == 3);
    }

    #[test]
    fn test_classify_lethal_rending() {
        let rolls = vec![1, 3, 3, 5, 5];
        let special = WeaponRules{punishing: false, rending: true, severe: false, lethal: 5};
        let result = classify_rolls(&rolls, 3, Some(&special));
        assert_eq!(result, SimResult{
            misses: 1,
            normals: 1,
            crits: 3,
        });

        assert!(result.hits() == 4);
    }

    #[test]
    fn test_simulate_accurate() {
        // accurate=2: roll 3 dice, retain 2 as normals after classify
        // rending fires on rolled dice only, then accurate normals are added
        // seed 8 first roll: [4, 3, 6] → miss=1(3<3? no, 3>=3), wait threshold=3
        // [4,3,6] threshold 3: 4=normal, 3=normal, 6=crit → rending: normal=1,crit=2 → +2 normals → normal=3,crit=2
        let special = WeaponRules{punishing: false, rending: true, severe: false, lethal: 6};
        let result = simulate_rolls(5, 3, &Reroll::None, &special, &Retain{ normals: 2, crits: 0 }, 1, &mut seeded_rng());
        assert_eq!(result, vec![SimResult{ misses: 0, normals: 3, crits: 2 }]);
    }

    // [4, 3], [6, 5], [4, 6], [4, 3], [5, 4]
    #[test]
    fn test_simulate_rolls() {
        let special = WeaponRules{punishing: false, rending: false, severe: false, lethal: 6};
        let result = simulate_rolls(2, 4, &Reroll::None, &special, &Retain{ normals: 0, crits: 0 }, 5, &mut seeded_rng());
        assert_eq!(result, vec![
            SimResult{ misses: 1, normals: 1, crits: 0, },
            SimResult{ misses: 0, normals: 1, crits: 1, },
            SimResult{ misses: 0, normals: 1, crits: 1, },
            SimResult{ misses: 1, normals: 1, crits: 0, },
            SimResult{ misses: 0, normals: 2, crits: 0, },
        ]);
    }
}
