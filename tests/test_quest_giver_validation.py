"""Synthetic quest-provider reference contracts through the shared storage reader."""
from copy import deepcopy
from pathlib import Path
import tempfile
import unittest

from shared.snapshots import SnapshotStore, validate_snapshot


def group(count=1):
    return {"entries": {str(100 + i): True for i in range(min(count, 16))},
            "distinct_id_count": count, "truncated": count > 16}


def fixture():
    return {
        "schema_version": 1, "source": "fixture", "captured_at": "2025-01-01T00:00:00Z",
        "client": {"family": "forever", "variant": "live", "build": "70862", "interface": 16001},
        "character": {"name": "Example", "realm": "Test"},
        "sections": {"quests": {key: {"title": "Fixture", "level": 1}
            for key in ("7", "10", "4294967295")}, "quest_giver_references": {
            "coverage": "first_saved_quest_ids", "provider": "questiedb", "provider_mode": "source",
            "provider_contract_version": 1, "reported_quest_count": 3,
            "entries": {"7": {"status": "reported", "started_by": {
                "npcs": group(), "objects": group(0), "items": group(17)},
                "finished_by": {"npcs": group(16), "objects": group(4096)}},
                "10": {"status": "unavailable"}, "4294967295": {"status": "not_in_provider"}}}},
    }


class QuestGiverValidationTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.store = SnapshotStore(Path(temporary.name))

    def assert_invalid(self, observation):
        value = fixture()
        value["sections"]["quest_giver_references"] = observation
        with self.assertRaises(ValueError):
            self.store.ingest(value)

    def test_storage_roundtrip_and_provider_modes(self):
        for mode in ("source", "baked"):
            value = fixture()
            value["sections"]["quest_giver_references"]["provider_mode"] = mode
            key = self.store.ingest(value)
            self.assertEqual(self.store.read(key), value)
            self.assertTrue(self.store.inspect(key, "quest_giver_references")["available"])

    def test_optional_unknown_empty_and_zero_observations(self):
        for row in ({"status": "reported"}, {"status": "reported", "started_by": {}},
                    {"status": "reported", "finished_by": {}},
                    {"status": "reported", "started_by": {"items": group(0)}}):
            value = fixture()
            value["sections"]["quest_giver_references"]["entries"]["7"] = row
            self.assertEqual(self.store.read(self.store.ingest(value)), value)
        value = fixture()
        observation = value["sections"]["quest_giver_references"]
        observation.update(reported_quest_count=0, entries={})
        value["sections"]["quests"] = {}
        self.assertEqual(self.store.read(self.store.ingest(value)), value)
        value["sections"] = {}
        validate_snapshot(value)
        value["unavailable"] = {"quest_giver_references": "Provider not loaded"}
        validate_snapshot(value)

    def test_quest_cap_and_non_contiguous_ids(self):
        for count in (1, 63, 64, 65, 66, 4096):
            value = fixture()
            value["sections"]["quests"] = {str(5000 + i * 5): {"title": "Fixture", "level": 1}
                for i in reversed(range(count))}
            value["sections"]["quest_giver_references"].update(
                reported_quest_count=count,
                entries={str(5000 + i * 5): {"status": "reported"} for i in range(min(count, 64))})
            self.assertEqual(self.store.read(self.store.ingest(value)), value)

    def test_source_quests_required_and_bounded(self):
        value = fixture()
        del value["sections"]["quests"]
        with self.assertRaisesRegex(ValueError, "source quests"):
            self.store.ingest(value)
        for source in (None, [], True, {str(i): {} for i in range(1, 4098)}):
            value = fixture()
            value["sections"]["quests"] = source
            with self.subTest(source_type=type(source)), self.assertRaisesRegex(ValueError, "source quests"):
                self.store.ingest(value)
        for invalid in ("0", "01", "-1", "+1", "1.0", " 1", "4294967296", "١", "", 1, True):
            value = fixture()
            value["sections"]["quests"][invalid] = value["sections"]["quests"].pop("7")
            with self.subTest(key=invalid), self.assertRaisesRegex(ValueError, "source quests"):
                self.store.ingest(value)

    def test_source_count_and_reference_ids_must_match(self):
        value = fixture()
        value["sections"]["quests"]["11"] = {}
        with self.assertRaisesRegex(ValueError, "source quests"):
            self.store.ingest(value)
        value = fixture()
        value["sections"]["quests"]["8"] = value["sections"]["quests"].pop("7")
        with self.assertRaisesRegex(ValueError, "first saved quest IDs"):
            self.store.ingest(value)

    def test_source_prefix_requires_numeric_first_64(self):
        value = fixture()
        value["sections"]["quests"] = {str(i): {"title": "Fixture", "level": 1}
            for i in reversed(range(1, 67))}
        observation = value["sections"]["quest_giver_references"]
        observation.update(reported_quest_count=66,
                           entries={str(i): {"status": "reported"} for i in range(1, 65)})
        self.assertEqual(self.store.read(self.store.ingest(value)), value)
        observation["entries"]["65"] = observation["entries"].pop("64")
        with self.assertRaisesRegex(ValueError, "first saved quest IDs"):
            self.store.ingest(value)
        observation["entries"] = {key: {"status": "reported"}
                                  for key in sorted(value["sections"]["quests"])[:64]}
        with self.assertRaisesRegex(ValueError, "first saved quest IDs"):
            self.store.ingest(value)

    def test_group_boundaries_and_u32_ids(self):
        for count in (0, 1, 15, 16, 17, 4096):
            value = fixture()
            row = value["sections"]["quest_giver_references"]["entries"]["7"]
            row["started_by"]["npcs"] = group(count)
            validate_snapshot(value)
        row["started_by"]["npcs"] = {
            "entries": {"4294967295": True}, "distinct_id_count": 1, "truncated": False}
        validate_snapshot(value)

    def test_exact_root_fields_and_values(self):
        original = fixture()["sections"]["quest_giver_references"]
        for field in original:
            bad = deepcopy(original)
            del bad[field]
            with self.subTest(missing=field):
                self.assert_invalid(bad)
        invalid = {"coverage": (None, "all", True), "provider": (None, "questie", True),
                   "provider_mode": (None, "live", True, [], {}),
                   "provider_contract_version": (None, True, False, 0, -1, 2**32, 1.0, "1"),
                   "reported_quest_count": (None, True, False, -1, 4097, 3.0, "3"),
                   "entries": (None, [], True)}
        for field, values in invalid.items():
            for invalid_value in values:
                with self.subTest(field=field, value=invalid_value):
                    self.assert_invalid({**original, field: invalid_value})
        self.assert_invalid({**original, "extra": True})
        for bad in (None, [], True):
            self.assert_invalid(bad)

    def test_exact_entry_count(self):
        original = fixture()["sections"]["quest_giver_references"]
        for count in (0, 1, 2, 4, 64, 4096):
            self.assert_invalid({**original, "reported_quest_count": count})

    def test_canonical_positive_u32_keys_at_both_levels(self):
        for invalid in ("0", "01", "-1", "+1", "1.0", " 1", "1 ", "4294967296", "١", "", 1, True):
            with self.subTest(invalid=invalid):
                bad = fixture()["sections"]["quest_giver_references"]
                bad["entries"][invalid] = bad["entries"].pop("7")
                self.assert_invalid(bad)
                bad = fixture()["sections"]["quest_giver_references"]
                bad["entries"]["7"]["started_by"]["npcs"]["entries"] = {invalid: True}
                self.assert_invalid(bad)

    def test_exact_status_rows(self):
        for row in (None, [], True, {}, {"status": None}, {"status": []}, {"status": "unknown"},
                    {"status": "reported", "extra": 1},
                    {"status": "unavailable", "started_by": {}},
                    {"status": "not_in_provider", "finished_by": {}}):
            with self.subTest(row=row):
                bad = fixture()["sections"]["quest_giver_references"]
                bad["entries"]["7"] = row
                self.assert_invalid(bad)

    def test_relations_and_groups_reject_unknown_fields_and_nulls(self):
        for relation in ("started_by", "finished_by"):
            for invalid in (None, [], True, {"unknown": group()}):
                bad = fixture()["sections"]["quest_giver_references"]
                bad["entries"]["7"][relation] = invalid
                self.assert_invalid(bad)
        bad = fixture()["sections"]["quest_giver_references"]
        bad["entries"]["7"]["finished_by"]["items"] = group()
        self.assert_invalid(bad)
        for invalid in (None, [], True, {}, {**group(), "extra": True}):
            bad = fixture()["sections"]["quest_giver_references"]
            bad["entries"]["7"]["started_by"]["npcs"] = invalid
            self.assert_invalid(bad)

    def test_group_fields_types_counts_and_truncation(self):
        invalid = {"distinct_id_count": (None, True, False, -1, 4097, 1.0, "1", 0, 2),
                   "truncated": (None, 0, 1, "false", True),
                   "entries": (None, [], True, {}, {"100": False}, {"100": 1},
                               {"100": None}, {"100": True, "101": True})}
        for field, values in invalid.items():
            for invalid_value in values:
                with self.subTest(field=field, value=invalid_value):
                    bad = fixture()["sections"]["quest_giver_references"]
                    bad["entries"]["7"]["started_by"]["npcs"][field] = invalid_value
                    self.assert_invalid(bad)
        for field in group():
            bad = fixture()["sections"]["quest_giver_references"]
            del bad["entries"]["7"]["started_by"]["npcs"][field]
            self.assert_invalid(bad)
        for count, entries, truncated in ((17, group(15)["entries"], True),
                                           (17, group(16)["entries"], False),
                                           (16, group(16)["entries"], True)):
            bad = fixture()["sections"]["quest_giver_references"]
            bad["entries"]["7"]["started_by"]["npcs"] = {
                "distinct_id_count": count, "entries": entries, "truncated": truncated}
            self.assert_invalid(bad)

    def test_wrong_client_family(self):
        for family in ("retail", "classic"):
            value = fixture()
            value["client"]["family"] = family
            with self.assertRaisesRegex(ValueError, "require Forever"):
                self.store.ingest(value)


if __name__ == "__main__":
    unittest.main()
