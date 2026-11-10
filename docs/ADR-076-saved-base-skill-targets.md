# Saved base skill targets

The existing skill-line inspection can plan distance to a user-supplied base
rank for recorded professions, weapon skills and other skill lines. This
reuses owner-local snapshot selection and the existing filtered page; no new
collector, source, storage or gameplay action is introduced.

A target is an optional uint32 on SkillLinePage. Per returned row, distance is
max(target - recorded base rank, 0). Reaching the recorded cap and reaching
the target are separate facts. A target above the recorded cap is accepted
and explicitly flagged, without asserting that training can unlock it.
Bonuses and temporary points remain separate observations. Targets do not
change which rows match or how pages are selected. Missing sections retain
null rows; empty captures retain an empty list. Without a target, responses
retain their existing shape.

The feature makes no current training, complete skill coverage, effective rank
or simulation-input claim. Offline verification does not qualify native APIs.

Distance ordering (10 November): SkillLinePage also accepts order_by with the
existing skill_line_id default, points_to_observed_cap or points_to_target.
Distance sorts ascending with numeric skill ID for ties after same-row filtering
and before pagination. Target order requires an explicit target, including zero.
Matching counts retain their existing whole-filter meaning. Reached targets
sort first because their distance is zero; no training cost, feasibility or
optimal progression model is inferred. Missing and empty captures stay distinct.
