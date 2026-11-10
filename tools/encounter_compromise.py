"""Bounded minimax regret across complete saved encounter point estimates."""
from copy import deepcopy
from fractions import Fraction
import hashlib
import json
from math import isfinite
from time import monotonic

from tools.merge_forever_gear_windows import digest, integer, require

MAX_CANDIDATES = 4096


def options(args):
    enabled = getattr(args, 'include_encounter_compromise', False)
    offset = getattr(args, 'encounter_compromise_offset', 0)
    require(type(enabled) is bool)
    integer(offset, 0, MAX_CANDIDATES)
    require(enabled or offset == 0)
    return enabled, offset


def ratio(value):
    return {'numerator': str(value.numerator), 'denominator': str(value.denominator)}


class Compromise:
    def __init__(self, encounters, gain_field, *, deadline=None, candidate_limit=MAX_CANDIDATES):
        integer(encounters, 1, 8)
        integer(candidate_limit, 1, MAX_CANDIDATES)
        self.dimensions, self.field, self.limit = encounters, gain_field, candidate_limit
        self.deadline = deadline
        self.rows, self.vectors = {}, {}
        self.stopped = None

    def _expired(self):
        if self.deadline is not None and monotonic() >= self.deadline:
            self.stopped = 'analysis_time_budget_exhausted'
        return self.stopped is not None

    def add(self, row):
        if self._expired():
            return
        key = digest(row['simulator_gear_sha256'])
        gains = row[self.field]
        require(type(gains) is list and len(gains) == self.dimensions)
        require(all(type(value) in (int, float) for value in gains))
        try:
            require(all(isfinite(value) for value in gains))
        except OverflowError as error:
            raise ValueError('Invalid encounter point estimate') from error
        vector = tuple(Fraction(value) for value in gains)
        if key in self.vectors:
            require(self.vectors[key] == vector)
            return
        if len(self.vectors) == self.limit:
            self.stopped = 'candidate_budget_exhausted'
            self.rows.clear()
            return
        self.vectors[key] = vector
        self.rows[key] = deepcopy(row)

    def report(self, *, complete, offset, limit):
        require(type(complete) is bool)
        integer(offset, 0, MAX_CANDIDATES)
        integer(limit, 1, 20)
        self._expired()
        status = 'incomplete_saved_results' if not complete else self.stopped or 'complete_for_saved_point_estimates'
        result = {'status': status, 'comparison_basis': 'minimise_maximum_gap_from_each_encounter_best_eligible_saved_improvement',
            'candidate_limit': self.limit, 'eligible_distinct_candidate_count': None,
            'analysis_stop_reason': self.stopped, 'offset': offset, 'limit': limit,
            'reference_best_improvement_by_encounter_exact': None,
            'minimum_worst_regret_exact': None, 'optimal_tied_gear_set_count': None,
            'point_estimates_sha256': None, 'has_more': None, 'next_offset': None,
            'gear_sets': [], 'selection_order': 'worst_regret_ascending_then_simulator_gear_sha256',
            'encounter_weights_used': False, 'statistical_superiority_verified': False,
            'global_ranking_verified': False, 'best_in_slot_verified': False,
            'feasible_set_coverage_verified': False, 'game_execution': False}
        if status != 'complete_for_saved_point_estimates':
            return result
        best = [max(vector[index] for vector in self.vectors.values()) for index in range(self.dimensions)] if self.vectors else []
        scored = []
        for key, vector in self.vectors.items():
            if self._expired():
                break
            gaps = tuple(a-b for a, b in zip(best, vector))
            scored.append((max(gaps), key, gaps))
        scored.sort(key=lambda entry: (entry[0], entry[1]))
        selected = []
        for score, key, gaps in scored[offset:offset+limit]:
            selected.append({'worst_regret_exact': ratio(score),
                'regret_by_encounter_exact': [ratio(value) for value in gaps],
                'gear_set': deepcopy(self.rows[key])})
        canonical = [[key, [ratio(value) for value in self.vectors[key]]] for key in sorted(self.vectors)]
        fingerprint = hashlib.sha256(json.dumps(canonical, separators=(',', ':')).encode()).hexdigest()
        if self._expired():
            result.update(status=self.stopped, analysis_stop_reason=self.stopped)
            return result
        end = min(len(scored), offset+limit)
        result.update(eligible_distinct_candidate_count=len(scored),
            reference_best_improvement_by_encounter_exact=[ratio(value) for value in best],
            minimum_worst_regret_exact=ratio(scored[0][0]) if scored else None,
            optimal_tied_gear_set_count=sum(score == scored[0][0] for score, _, _ in scored) if scored else 0,
            point_estimates_sha256=fingerprint, has_more=end < len(scored),
            next_offset=end if end < len(scored) else None, gear_sets=selected)
        return result
