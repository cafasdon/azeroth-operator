"""Bounded exact Pareto membership over completed encounter point estimates."""
from copy import deepcopy
import hashlib
import json
from math import isfinite
from time import monotonic

from tools.merge_forever_gear_windows import digest, integer, require

MAX_CANDIDATES = 4096


def options(args):
    enabled = getattr(args, 'include_encounter_frontier', False)
    offset = getattr(args, 'encounter_frontier_offset', 0)
    require(type(enabled) is bool)
    integer(offset, 0, MAX_CANDIDATES)
    require(enabled or offset == 0)
    return enabled, offset


class Frontier:
    def __init__(self, encounters, gain_field, *, deadline=None, candidate_limit=MAX_CANDIDATES):
        integer(encounters, 1, 8)
        integer(candidate_limit, 1, MAX_CANDIDATES)
        require(deadline is None or type(deadline) in (int, float) and isfinite(deadline))
        self.encounters, self.gain_field = encounters, gain_field
        self.deadline, self.limit = deadline, candidate_limit
        self.seen, self.groups, self.rows = {}, {}, {}
        self.instances = 0
        self.stopped = None

    def _stop(self, reason):
        self.stopped = reason
        self.groups.clear()
        self.rows.clear()

    def add(self, row):
        gear = digest(row['simulator_gear_sha256'])
        gains = row[self.gain_field]
        require(type(gains) is list and len(gains) == self.encounters)
        require(all(type(value) in (int, float) for value in gains))
        try:
            require(all(isfinite(value) for value in gains))
        except OverflowError as error:
            raise ValueError('Invalid encounter point estimate') from error
        vector = tuple(gains)
        self.instances += 1
        if self.stopped:
            return
        if gear in self.seen:
            require(self.seen[gear] == vector)
            return
        if len(self.seen) == self.limit:
            self._stop('candidate_budget_exhausted')
            return
        self.seen[gear] = vector
        if self.deadline is not None and monotonic() >= self.deadline:
            self._stop('analysis_time_budget_exhausted')
            return
        if vector in self.groups:
            self.groups[vector].append(gear)
        else:
            dominated = []
            for index, previous in enumerate(self.groups):
                if index % 64 == 0 and self.deadline is not None and monotonic() >= self.deadline:
                    self._stop('analysis_time_budget_exhausted')
                    return
                if all(a >= b for a, b in zip(previous, vector)):
                    return  # Unequal vectors: at least one comparison is strict.
                if all(a >= b for a, b in zip(vector, previous)):
                    dominated.append(previous)
            for previous in dominated:
                for old in self.groups.pop(previous):
                    self.rows.pop(old)
            self.groups[vector] = [gear]
        self.rows[gear] = deepcopy(row)

    def report(self, *, complete, offset, limit):
        require(type(complete) is bool)
        integer(offset, 0, MAX_CANDIDATES)
        integer(limit, 1, 20)
        if not self.stopped and self.deadline is not None and monotonic() >= self.deadline:
            self._stop('analysis_time_budget_exhausted')
        status = 'incomplete_saved_results' if not complete else self.stopped or 'complete_for_saved_point_estimates'
        known = status == 'complete_for_saved_point_estimates'
        keys = sorted(self.rows) if known else []
        end = min(len(keys), offset + limit)
        fingerprint = None
        if known:
            canonical = [[key, [value.as_integer_ratio() for value in self.seen[key]]] for key in keys]
            fingerprint = hashlib.sha256(json.dumps(canonical, separators=(',', ':'), allow_nan=False).encode()).hexdigest()
            if self.deadline is not None and monotonic() >= self.deadline:
                self._stop('analysis_time_budget_exhausted')
                status, known, keys, end, fingerprint = self.stopped, False, [], 0, None
        return {'status': status, 'comparison_basis': 'no_lower_improvement_in_every_encounter_and_strictly_higher_in_one',
            'eligible_candidate_instances_from_completed_pages': self.instances,
            'eligible_distinct_candidate_count': len(self.seen) if known else None,
            'candidate_limit': self.limit, 'analysis_stop_reason': self.stopped,
            'frontier_gear_set_count': len(keys) if known else None,
            'frontier_point_estimates_sha256': fingerprint,
            'offset': offset, 'limit': limit, 'has_more': end < len(keys) if known else None,
            'next_offset': end if known and end < len(keys) else None,
            'selection_order': 'simulator_gear_sha256_ascending',
            'gear_sets': [self.rows[key] for key in keys[offset:end]],
            'tied_vectors_keep_distinct_gear_sets': True, 'statistical_superiority_verified': False,
            'global_frontier_verified': False, 'best_in_slot_verified': False,
            'feasible_set_coverage_verified': False, 'game_execution': False}
