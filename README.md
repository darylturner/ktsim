# ktsim

Monte Carlo dice simulator for Kill Team attack rolls.

Simulates rolling a pool of d6s against a hit threshold, with support for reroll abilities, critical hit rules, and special weapon traits. Run thousands of simulations to see the full probability distribution of outcomes.

## Install

```
cargo install --git https://github.com/darylturner/ktsim
```

## Build

```
cargo build --release
```

## Usage

```
ktsim [OPTIONS]
```

### Options

| Flag            | Short | Default | Description                                                  |
|-----------------|-------|---------|--------------------------------------------------------------|
| `--attacks N`   | `-a`  | 4       | Attack characteristic of the weapon                          |
| `--hit N`       | `-H`  | 3       | Hit characteristic of the weapon (2–6)                       |
| `--lethal N`    | `-l`  | 6       | Roll this value or higher for a critical hit (2–6)           |
| `--reroll TYPE` | `-r`  | none    | Reroll ability (see below)                                   |
| `--punishing`   |       | off     | If any critical rolled, convert one miss to a normal hit     |
| `--rending`     |       | off     | If any critical rolled, convert one normal hit to a critical |
| `--severe`      |       | off     | If no criticals rolled, convert one normal hit to a critical |
| `--accurate N`  |       | 0       | Retain N dice as normal hits without rolling them (0–2)      |
| `--sims N`      | `-s`  | 10,000  | Number of simulations to run                                 |
| `--output TYPE` | `-o`  | hits    | Output format: `hits` or `full`                              |

### Reroll abilities

| Value        | Description                                              |
|--------------|----------------------------------------------------------|
| `none`       | No rerolls                                               |
| `balanced`   | Reroll 1 miss                                            |
| `ceaseless`  | Reroll all misses sharing the most frequent missed value |
| `relentless` | Reroll all misses                                        |

### Special rules interaction

Punishing, rending, and severe are applied in that order after rerolls. Severe only fires when there are no criticals, so it cannot trigger punishing or rending. Accurate dice are retained before rerolls and are not subject to weapon effects.

## Output

### `--output hits` (default)

Shows the distribution of total hit counts across all simulations, with probability and cumulative probability columns.

```
  Hits   Count       Prob   ≥ Prob   Distribution
  0      113         1.1%   100.0%
  1      991         9.9%    98.9%   ██████
  2      2925       29.2%    89.0%   ████████████████████
  3      3982       39.8%    59.7%   ████████████████████████████ ◄ mean
  4      1989       19.9%    19.9%   █████████████
```

The `≥ Prob` column shows the probability of getting **at least** that many hits.

### `--output full`

Shows every unique combination of crits, normals, and misses with its probability. Useful for understanding the crit distribution and the effect of special rules.

```
  Hits  Crits  Normals  Misses   Count       Prob   ≥ Prob   Distribution
  0     0      0        4        113         1.1%   100.0%   █
  1     0      1        3        733         7.3%    98.9%   ████████
  1     1      0        3        258         2.6%    98.9%   ██
  ...
```

## Examples

4 attacks, 3+ to hit, balanced reroll:
```
ktsim -a 4 -H 3 -r balanced
```

5 attacks, 4+ to hit, lethal 5, relentless rerolls:
```
ktsim -a 5 -H 4 -l 5 -r relentless
```

4 attacks, 3+ to hit, rending and punishing, full breakdown:
```
ktsim -a 4 -H 3 --rending --punishing -o full
```

## Credits

ASCII art skull from [ascii.co.uk/art/skulls](https://ascii.co.uk/art/skulls), artist unknown.
