#!/usr/bin/env python3
"""Simulate remote-IM group reply pacing and select conservative debounce defaults.

This is a deterministic Monte Carlo calibration tool.  Direct calls are a
forced-response branch outside this model.  The simulator calibrates natural
replies: they spend energy, recover over time, and never become impossible.
"""

from __future__ import annotations

import argparse
import heapq
import math
import random
from dataclasses import dataclass
from itertools import product
from statistics import fmean


SECONDS_PER_HOUR = 60 * 60
SIMULATION_HOURS = 8
RUNS_PER_SCENARIO = 30


@dataclass(frozen=True)
class Parameters:
    secretary_debounce_seconds: float = 7.0
    reply_cooldown_seconds: float = 30.0
    fluctuation_threshold: float = 0.2
    enthusiasm: float = 0.3
    maximum_energy: float = 100.0
    base_reply_energy_cost: float = 12.0
    energy_cost_per_character: float = 0.16
    energy_recovery_min_per_second: float = 0.02
    energy_recovery_max_per_second: float = 0.20
    positive_word_energy_ratio: float = 0.08
    negative_word_energy_ratio: float = 0.12

    def energy_recovery_per_second(self) -> float:
        return self.energy_recovery_min_per_second + (
            self.energy_recovery_max_per_second - self.energy_recovery_min_per_second
        ) * self.enthusiasm

    def positive_word_energy_delta(self) -> float:
        return self.maximum_energy * self.positive_word_energy_ratio

    def negative_word_energy_delta(self) -> float:
        return self.maximum_energy * self.negative_word_energy_ratio

    def reply_energy_cost(self, character_count: int) -> float:
        return self.base_reply_energy_cost + max(0, character_count) * self.energy_cost_per_character

@dataclass(frozen=True)
class Scenario:
    name: str
    message_rate_per_hour: float
    reply_worthy_rate: float
    target_replies_per_hour: float
    max_replies_per_hour: float
    positive_word_rate: float = 0.04
    negative_word_rate: float = 0.03
    bursts: tuple[tuple[float, int, float, float], ...] = ()
    """(hour from start, message count, duration seconds, reply-worthy rate)."""


SCENARIOS = (
    Scenario("安静群", 3.0, 0.25, 0.6, 1.5),
    Scenario("普通讨论", 24.0, 0.16, 3.5, 5.0),
    Scenario("活跃讨论", 90.0, 0.13, 6.0, 8.0),
    Scenario(
        "短时爆发",
        20.0,
        0.14,
        4.0,
        6.0,
        bursts=(
            (1.0, 36, 90.0, 0.20),
            (3.0, 42, 120.0, 0.20),
            (5.5, 30, 75.0, 0.20),
        ),
    ),
)


@dataclass
class SimulationResult:
    replies: int
    reply_times: list[float]
    reply_character_counts: list[int]
    cooldown_restarts: int

    def replies_per_hour(self, duration_seconds: float) -> float:
        return self.replies / (duration_seconds / SECONDS_PER_HOUR)

    def shortest_gap_seconds(self) -> float:
        if len(self.reply_times) < 2:
            return math.inf
        return min(
            right - left for left, right in zip(self.reply_times, self.reply_times[1:])
        )

    def average_reply_characters(self) -> float:
        return fmean(self.reply_character_counts) if self.reply_character_counts else 0.0


@dataclass
class ScenarioSummary:
    scenario: Scenario
    replies_per_hour: float
    shortest_gap_seconds: float
    cooldown_restarts_per_hour: float
    average_reply_characters: float


@dataclass
class CandidateSummary:
    parameters: Parameters
    score: float
    scenarios: list[ScenarioSummary]


def poisson_message_times(
    rng: random.Random,
    rate_per_hour: float,
    duration_seconds: float,
) -> list[float]:
    if rate_per_hour <= 0:
        return []
    times: list[float] = []
    moment = 0.0
    rate_per_second = rate_per_hour / SECONDS_PER_HOUR
    while True:
        moment += rng.expovariate(rate_per_second)
        if moment >= duration_seconds:
            return times
        times.append(moment)


def create_message_events(
    scenario: Scenario,
    duration_seconds: float,
    rng: random.Random,
) -> list[tuple[float, bool, bool, bool]]:
    events = [
        (
            moment,
            rng.random() < scenario.reply_worthy_rate,
            rng.random() < scenario.positive_word_rate,
            rng.random() < scenario.negative_word_rate,
        )
        for moment in poisson_message_times(rng, scenario.message_rate_per_hour, duration_seconds)
    ]
    for burst_hour, count, burst_duration, reply_worthy_rate in scenario.bursts:
        start = burst_hour * SECONDS_PER_HOUR
        for _ in range(count):
            events.append((
                start + rng.uniform(0.0, burst_duration),
                rng.random() < reply_worthy_rate,
                rng.random() < scenario.positive_word_rate,
                rng.random() < scenario.negative_word_rate,
            ))
    return sorted(events)


def draw_reply_character_count(rng: random.Random) -> int:
    """Model terse group-chat replies, with rare longer explanations."""
    roll = rng.random()
    if roll < 0.85:
        return round(rng.triangular(4.0, 20.0, 10.0))
    if roll < 0.98:
        return round(rng.triangular(21.0, 80.0, 35.0))
    return round(rng.triangular(81.0, 200.0, 120.0))


def recovered_energy(
    energy: float,
    energy_recorded_at: float | None,
    now: float,
    parameters: Parameters,
) -> float:
    if energy_recorded_at is None or energy >= parameters.maximum_energy:
        return min(parameters.maximum_energy, max(0.0, energy))
    elapsed = max(0.0, now - energy_recorded_at)
    return min(
        parameters.maximum_energy,
        energy + elapsed * parameters.energy_recovery_per_second(),
    )


def debounce_delay_seconds(rng: random.Random, parameters: Parameters) -> float:
    fluctuation = rng.uniform(
        -parameters.fluctuation_threshold,
        parameters.fluctuation_threshold,
    )
    base_with_fluctuation = parameters.secretary_debounce_seconds * (1.0 + fluctuation)
    return max(1.0, base_with_fluctuation)


def simulate(
    parameters: Parameters,
    scenario: Scenario,
    seed: int,
    duration_seconds: float = SIMULATION_HOURS * SECONDS_PER_HOUR,
) -> SimulationResult:
    rng = random.Random(seed)
    events = create_message_events(scenario, duration_seconds, rng)
    event_index = 0
    now = 0.0
    pending_reply_worthy = False
    secretary_due_at: float | None = None
    energy = parameters.maximum_energy
    energy_recorded_at: float | None = None
    cooldown_until = 0.0
    reply_times: list[float] = []
    reply_character_counts: list[int] = []
    cooldown_restarts = 0

    while event_index < len(events) or secretary_due_at is not None:
        next_message_at = events[event_index][0] if event_index < len(events) else math.inf
        next_due_at = secretary_due_at if secretary_due_at is not None else math.inf
        if next_message_at <= next_due_at:
            now, reply_worthy, positive_word_hit, negative_word_hit = events[event_index]
            event_index += 1
            energy = recovered_energy(energy, energy_recorded_at, now, parameters)
            if positive_word_hit:
                energy += parameters.positive_word_energy_delta()
            if negative_word_hit:
                energy -= parameters.negative_word_energy_delta()
            energy = min(parameters.maximum_energy, max(0.0, energy))
            energy_recorded_at = now
            pending_reply_worthy = pending_reply_worthy or reply_worthy
            if secretary_due_at is None:
                secretary_due_at = now + debounce_delay_seconds(rng, parameters)
            continue

        now = next_due_at
        if not pending_reply_worthy:
            secretary_due_at = None
            continue
        current_energy = recovered_energy(energy, energy_recorded_at, now, parameters)
        if now < cooldown_until:
            # Keep the unresolved batch.  A completed secretary debounce starts
            # another debounce while cooling down; messages continue accumulating.
            cooldown_restarts += 1
            secretary_due_at = now + debounce_delay_seconds(rng, parameters)
            continue

        character_count = draw_reply_character_count(rng)
        reply_energy_cost = parameters.reply_energy_cost(character_count)
        if current_energy < reply_energy_cost:
            # Natural conversation is a skill: leave the relevant batch pending
            # and try again after more energy has recovered.
            secretary_due_at = now + debounce_delay_seconds(rng, parameters)
            continue

        reply_times.append(now)
        reply_character_counts.append(character_count)
        energy = max(0.0, current_energy - reply_energy_cost)
        energy_recorded_at = now
        cooldown_until = now + parameters.reply_cooldown_seconds
        pending_reply_worthy = False
        secretary_due_at = None

    return SimulationResult(
        replies=len(reply_times),
        reply_times=reply_times,
        reply_character_counts=reply_character_counts,
        cooldown_restarts=cooldown_restarts,
    )


def summarize(parameters: Parameters, scenario: Scenario, runs: int) -> ScenarioSummary:
    duration_seconds = SIMULATION_HOURS * SECONDS_PER_HOUR
    results = [simulate(parameters, scenario, seed=index) for index in range(runs)]
    finite_gaps = [item.shortest_gap_seconds() for item in results if math.isfinite(item.shortest_gap_seconds())]
    return ScenarioSummary(
        scenario=scenario,
        replies_per_hour=fmean(item.replies_per_hour(duration_seconds) for item in results),
        shortest_gap_seconds=fmean(finite_gaps) if finite_gaps else math.inf,
        cooldown_restarts_per_hour=fmean(item.cooldown_restarts for item in results) / SIMULATION_HOURS,
        average_reply_characters=fmean(item.average_reply_characters() for item in results),
    )


def score_summaries(summaries: list[ScenarioSummary]) -> float:
    score = 0.0
    for item in summaries:
        target = item.scenario.target_replies_per_hour
        value = item.replies_per_hour
        score += abs(value - target) / max(target, 0.5)
        if value > item.scenario.max_replies_per_hour:
            score += 8.0 * (value - item.scenario.max_replies_per_hour)
        if item.shortest_gap_seconds < 10.0:
            score += (10.0 - item.shortest_gap_seconds) / 2.0
    return score


def candidate_grid() -> list[Parameters]:
    candidates: list[Parameters] = []
    for cooldown, base_cost, per_character_cost, positive_ratio, negative_ratio in product(
        (10.0, 12.0, 15.0),
        (8.0, 10.0, 12.0, 14.0),
        (0.06, 0.08, 0.10, 0.12),
        (0.06, 0.08),
        (0.10, 0.12, 0.15),
    ):
        candidates.append(Parameters(
            reply_cooldown_seconds=cooldown,
            base_reply_energy_cost=base_cost,
            energy_cost_per_character=per_character_cost,
            positive_word_energy_ratio=positive_ratio,
            negative_word_energy_ratio=negative_ratio,
        ))
    return candidates


def calibrate(top_n: int, runs: int) -> list[CandidateSummary]:
    ranked: list[CandidateSummary] = []
    for parameters in candidate_grid():
        summaries = [summarize(parameters, scenario, runs) for scenario in SCENARIOS]
        ranked.append(CandidateSummary(parameters, score_summaries(summaries), summaries))
    return heapq.nsmallest(top_n, ranked, key=lambda item: item.score)


def print_candidate(candidate: CandidateSummary, heading: str) -> None:
    parameters = candidate.parameters
    print(heading)
    print(
        "  "
        f"score={candidate.score:.2f}, cooldown={parameters.reply_cooldown_seconds:.0f}s, "
        f"enthusiasm={parameters.enthusiasm:.1f}, "
        f"recovery={parameters.energy_recovery_per_second():.3f}/s, "
        f"cost=base {parameters.base_reply_energy_cost:.0f} + {parameters.energy_cost_per_character:.2f}/字, "
        f"positive=+{parameters.positive_word_energy_delta():.0f}, "
        f"negative=-{parameters.negative_word_energy_delta():.0f}"
    )
    print("  场景          回复/小时  平均字数  最短间隔(秒)  冷却重跑/小时")
    for item in candidate.scenarios:
        gap = "无" if math.isinf(item.shortest_gap_seconds) else f"{item.shortest_gap_seconds:.0f}"
        print(
            f"  {item.scenario.name:<8}  {item.replies_per_hour:>8.2f}"
            f"  {item.average_reply_characters:>8.0f}  {gap:>12}"
            f"  {item.cooldown_restarts_per_hour:>13.2f}"
        )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--top", type=int, default=5, help="Number of parameter candidates to print.")
    parser.add_argument("--runs", type=int, default=RUNS_PER_SCENARIO, help="Monte Carlo runs per scenario.")
    args = parser.parse_args()
    if args.top < 1:
        parser.error("--top must be positive")
    if args.runs < 1:
        parser.error("--runs must be positive")

    baseline = CandidateSummary(
        Parameters(
            reply_cooldown_seconds=0.0,
            enthusiasm=1.0,
            base_reply_energy_cost=0.0,
            energy_cost_per_character=0.0,
        ),
        0.0,
        [],
    )
    baseline.scenarios = [summarize(baseline.parameters, scenario, args.runs) for scenario in SCENARIOS]
    baseline.score = score_summaries(baseline.scenarios)
    print_candidate(baseline, "无冷却、无累积降速（对照组）")
    for index, candidate in enumerate(calibrate(args.top, args.runs), start=1):
        print()
        print_candidate(candidate, f"候选 {index}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
