// Which spent talents the game would not allow with the points around them.
//
// A talent in row N needs 5 * N points in the rows above it, and a talent with a prerequisite needs that
// prerequisite at its maximum rank. A broken talent's points stay in the string, so refilling the gap
// brings the build back, but they unlock nothing: rows below count only the points of talents that are
// not broken, and a broken prerequisite breaks its children.

import type { TalentsConfig } from './config';
import { locationKey } from './config';
import type { TalentPoints } from './talents_string';

const POINTS_PER_ROW = 5;

/** `[treeIdx][talentIdx]`: true where the talent holds points it could not have been given. */
export const findBrokenTalents = <TalentsProto>(config: TalentsConfig<TalentsProto>, points: TalentPoints): Array<Array<boolean>> =>
	config.map((tree, treeIdx) => {
		const talents = tree.talents;
		const treePoints = points[treeIdx];
		const idxByLocation = new Map(talents.map((talent, idx) => [locationKey(talent.location), idx]));
		const broken = talents.map(() => false);
		const decided = talents.map(() => false);
		let activeAbove = 0;

		// A prerequisite sits in an earlier row or on either side in the same row, so it is always decided
		// against the same or a smaller `activeAbove` than its child.
		const decide = (idx: number): boolean => {
			if (decided[idx]) return broken[idx];
			decided[idx] = true;
			if (treePoints[idx] === 0) return false;

			const talent = talents[idx];
			const prereqIdx = talent.prereqLocation ? idxByLocation.get(locationKey(talent.prereqLocation)) : undefined;
			broken[idx] =
				activeAbove < talent.location.rowIdx * POINTS_PER_ROW ||
				(prereqIdx !== undefined && (decide(prereqIdx) || treePoints[prereqIdx] < talents[prereqIdx].maxPoints));
			return broken[idx];
		};

		// newTalentsConfig rejects a tree not listed row by row, so one pass sees each row whole.
		let row = 0;
		let rowActive = 0;
		talents.forEach((talent, idx) => {
			if (talent.location.rowIdx !== row) {
				activeAbove += rowActive;
				rowActive = 0;
				row = talent.location.rowIdx;
			}
			if (!decide(idx)) rowActive += treePoints[idx];
		});
		return broken;
	});

/** The display names of every broken talent, tree by tree. */
export const brokenTalentNames = <TalentsProto>(config: TalentsConfig<TalentsProto>, points: TalentPoints): Array<string> =>
	findBrokenTalents(config, points).flatMap((tree, treeIdx) => tree.flatMap((broken, idx) => (broken ? [config[treeIdx].talents[idx].fancyName] : [])));
