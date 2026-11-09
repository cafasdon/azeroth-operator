"""Validate role metrics and compare their native improvements across encounters."""
from math import fsum, isclose, isfinite


FIELDS = {
    'dps': 'mean_dps',
    'dtps': 'mean_dtps',
    'tps': 'mean_tps',
    'death': 'simulated_death_fraction',
    'hps': 'mean_hps',
    'oom': 'mean_seconds_oom',
}
UNITS = {
    'dps': 'percent',
    'dtps': 'damage_per_second',
    'tps': 'threat_per_second',
    'death': 'percentage_points',
    'hps': 'healing_per_second',
    'oom': 'seconds',
}


def _require(condition, message):
    if not condition:
        raise ValueError(message)


def _metric(metric):
    _require(type(metric) is str and metric in FIELDS, 'Unknown ranking metric')
    return metric


def rank_by(args):
    """Resolve the optional ranking argument without accessing external inputs."""
    return _metric(getattr(args, 'rank_by', 'dps'))


def _number(number):
    _require(type(number) in (int, float), 'Metric values and weights must be numeric')
    try:
        number = float(number)
    except OverflowError as exc:
        raise ValueError('Metric values and weights must be finite') from exc
    _require(isfinite(number), 'Metric values and weights must be finite')
    return number


def value(row, metric):
    """Read one finite metric; only threat is permitted to be negative."""
    field = FIELDS[_metric(metric)]
    _require(type(row) is dict and field in row, 'Missing ranking metric')
    result = _number(row[field])
    _require(metric == 'tps' or result >= 0, 'Ranking metric cannot be negative')
    _require(metric != 'death' or result <= 1, 'Death fraction must be between zero and one')
    return result


def _review(review):
    _require(type(review) is int and review >= 1, 'Review IDs must be positive integers')
    return review


def weighted_ranking(rankings, weights, metric):
    """Rank complete encounters by weighted native improvement, never DPS ratios.

    Encounter order is retained in the numeric detail arrays. Inputs are read
    only, and output contains no source identity or arbitrary input metadata.
    """
    metric = _metric(metric)
    _require(metric != 'dps', 'DPS uses the legacy percentage ranking')
    _require(type(rankings) is list and 1 <= len(rankings) <= 8,
             'Expected one to eight encounter rankings')
    _require(type(weights) in (list, tuple) and len(weights) == len(rankings),
             'Expected one weight per encounter')
    weights = [_number(weight) for weight in weights]
    _require(all(weight > 0 for weight in weights), 'Encounter weights must be positive')
    try:
        total = fsum(weights)
    except OverflowError as exc:
        raise ValueError('Encounter weight sum must be finite') from exc
    _require(isfinite(total) and isclose(total, 1.0, rel_tol=1e-9, abs_tol=1e-12),
             'Encounter weights must be normalised')

    profiles = []
    expected_ids = baseline_id = None
    for ranking in rankings:
        _require(type(ranking) is dict, 'Invalid encounter ranking')
        _require('baseline_review' in ranking, 'Missing baseline review')
        baseline = _review(ranking['baseline_review'])
        _require('requested_results_complete' not in ranking
                 or ranking['requested_results_complete'] is True, 'Encounter ranking is incomplete')
        rows = ranking.get('results')
        _require(type(rows) is list and bool(rows), 'Missing encounter results')
        profile = {}
        for row in rows:
            _require(type(row) is dict and 'review' in row, 'Missing review ID')
            review = _review(row['review'])
            _require(review not in profile, 'Duplicate review ID in encounter')
            profile[review] = value(row, metric)
        _require(baseline in profile, 'Baseline review is absent from encounter')
        if expected_ids is None:
            expected_ids, baseline_id = set(profile), baseline
        _require(set(profile) == expected_ids and baseline == baseline_id,
                 'Encounters must contain the same reviews and baseline')
        profiles.append(profile)

    results = []
    for review in expected_ids:
        values, improvements, weighted = [], [], []
        for profile, weight in zip(profiles, weights, strict=True):
            observed, baseline = profile[review], profile[baseline_id]
            improvement = (observed - baseline if metric in ('tps', 'hps')
                           else baseline - observed)
            if metric == 'death':
                improvement *= 100
            _require(isfinite(improvement), 'Metric improvement overflowed')
            contribution = weight * improvement
            _require(isfinite(contribution), 'Weighted improvement overflowed')
            values.append(observed)
            improvements.append(improvement)
            weighted.append(contribution)
        try:
            score = fsum(weighted)
        except OverflowError as exc:
            raise ValueError('Weighted improvement sum overflowed') from exc
        _require(isfinite(score), 'Weighted improvement sum overflowed')
        results.append({'review': review, 'weighted_improvement': score,
                        'improvement_by_encounter': improvements,
                        'metric_values_by_encounter': values})
    results.sort(key=lambda row: (-row['weighted_improvement'], row['review']))
    return {'rank_by': metric, 'score_unit': UNITS[metric], 'results': results}
