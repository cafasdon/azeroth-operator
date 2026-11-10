"""Strict numeric inbox observations never include private mail content."""
import copy
import hashlib
import json
from pathlib import Path
import tempfile
import unittest

from shared.snapshots import SnapshotStore, validate_snapshot


def fixture():
    return {
        "schema_version": 1,
        "client": {"family": "forever", "variant": "live", "build": "70862", "interface": 16001},
        "character": {"name": "FundsFixture", "realm": "FixtureRealm"},
        "source": "fixture", "captured_at": "2025-01-01T00:00:00Z",
        "sections": {
            "wallet": {"copper": 5},
            "mail_funds": {
                "coverage": "loaded_inbox_funds", "loaded_messages": 3, "total_messages": 5,
                "messages": {
                    "1": {"status": "reported", "money_copper": 100, "cod_copper": 0},
                    "2": {"status": "reported", "money_copper": 0, "cod_copper": 20},
                    "3": {"status": "unavailable"},
                },
            },
        },
    }


class MailFundsValidationTests(unittest.TestCase):
    def test_partial_observation_is_source_independent_and_unchanged(self):
        for source in ("fixture", "addon", "official_api"):
            value = fixture()
            value["source"] = source
            before = copy.deepcopy(value)
            self.assertIs(validate_snapshot(value), value)
            self.assertEqual(value, before)
            self.assertNotIn("mail_attachments", value["sections"])

    def test_reported_zero_high_values_empty_and_maximum_coverage(self):
        for copper in (0, 9_999_999_999_999):
            value = fixture()
            funds = value["sections"]["mail_funds"]
            funds["total_messages"] = funds["loaded_messages"]
            funds["messages"] = {str(i): {"status": "reported", "money_copper": copper,
                                         "cod_copper": copper} for i in range(1, 4)}
            validate_snapshot(value)
        for loaded, total in ((0, 0), (0, 5), (100, 4_294_967_295)):
            value = fixture()
            value["sections"]["mail_funds"].update(
                loaded_messages=loaded, total_messages=total,
                messages={str(i): {"status": "unavailable"} for i in range(1, loaded + 1)})
            validate_snapshot(value)

    def test_missing_and_unavailable_section(self):
        value = fixture()
        value["sections"].pop("mail_funds")
        validate_snapshot(value)
        value["unavailable"] = {"mail_funds": "inbox_not_open"}
        validate_snapshot(value)
        for reason in (None, "", " ", 0, "x" * 257):
            with self.subTest(reason=reason):
                value["unavailable"]["mail_funds"] = reason
                with self.assertRaises(ValueError):
                    validate_snapshot(value)
        value = fixture()
        value["unavailable"] = {"mail_funds": "inbox_not_open"}
        with self.assertRaises(ValueError):
            validate_snapshot(value)

    def test_rejects_wrong_family_and_source(self):
        for family in ("retail", "classic"):
            value = fixture()
            value["sections"].pop("wallet")
            value["client"]["family"] = family
            with self.subTest(family=family), self.assertRaises(ValueError):
                validate_snapshot(value)
        value = fixture()
        value["source"] = "mail"
        with self.assertRaises(ValueError):
            validate_snapshot(value)

    def test_rejects_coverage_types_keys_and_privacy_fields(self):
        cases = [{"coverage": value} for value in (None, True, "loaded_inbox_attachments")]
        cases += [{"loaded_messages": value} for value in (-1, 101, True, 3.0, "3", None)]
        cases += [{"total_messages": value} for value in (2, 4_294_967_296, True, 5.0, "5", None)]
        cases += [{"messages": value} for value in (None, [], {}, {"01": {"status": "unavailable"}},
                   {1: {"status": "unavailable"}}, {"1": {}, "2": {}, "4": {}},
                   {"1": {}, "2": {}, "3": {}, "4": {}})]
        cases += [{field: "private"} for field in ("sender", "subject", "body", "recipient", "item_id")]
        for change in cases:
            value = fixture()
            value["sections"]["mail_funds"].update(change)
            with self.subTest(change=change), self.assertRaises(ValueError):
                validate_snapshot(value)
        for field in fixture()["sections"]["mail_funds"]:
            value = fixture()
            del value["sections"]["mail_funds"][field]
            with self.subTest(missing=field), self.assertRaises(ValueError):
                validate_snapshot(value)
        for funds in (None, [], 1):
            value = fixture()
            value["sections"]["mail_funds"] = funds
            with self.subTest(funds=funds), self.assertRaises(ValueError):
                validate_snapshot(value)

    def test_rejects_row_types_statuses_amounts_and_extra_fields(self):
        rows = [None, [], 0, {}]
        rows += [{"status": status} for status in (None, True, "unknown", "available", "reported")]
        reported = {"status": "reported", "money_copper": 0, "cod_copper": 0}
        for field in ("money_copper", "cod_copper"):
            rows += [{**reported, field: amount} for amount in
                     (-1, 10_000_000_000_000, True, False, 0.0, "0", None)]
            missing = dict(reported)
            del missing[field]
            rows.append(missing)
            rows.append({"status": "unavailable", field: 0})
        for field in ("sender", "subject", "body", "recipient", "item_id", "attachment_slot"):
            rows += [{**reported, field: "private"}, {"status": "unavailable", field: "private"}]
        for row in rows:
            value = fixture()
            value["sections"]["mail_funds"]["messages"]["1"] = row
            with self.subTest(row=row), self.assertRaises(ValueError):
                validate_snapshot(value)

    def test_storage_inspection_and_payload_byte_immutability(self):
        value = fixture()
        before = json.dumps(value, sort_keys=True).encode()
        with tempfile.TemporaryDirectory() as directory:
            store = SnapshotStore(Path(directory))
            key = store.ingest(value)
            path = Path(directory) / (key + ".json")
            raw = path.read_bytes()
            self.assertEqual(hashlib.sha256(raw).hexdigest(), key)
            self.assertEqual(store.read(key), value)
            self.assertEqual(store.inspect(key, "mail_funds")["data"], value["sections"]["mail_funds"])
            self.assertTrue(store.inspect(key, "mail_funds")["available"])
            self.assertIn("mail_funds", store.inspect(key)["available_sections"])
            self.assertEqual(store.ingest(value), key)
            self.assertEqual(path.read_bytes(), raw)
            self.assertEqual(json.dumps(value, sort_keys=True).encode(), before)
            missing = fixture()
            missing["sections"].pop("mail_funds")
            missing_key = store.ingest(missing)
            inspected = store.inspect(missing_key, "mail_funds")
            self.assertFalse(inspected["available"])
            self.assertIsNone(inspected["data"])
            invalid = fixture()
            invalid["sections"]["mail_funds"]["messages"]["1"]["subject"] = "private"
            bad_raw = json.dumps(invalid).encode()
            bad_key = hashlib.sha256(bad_raw).hexdigest()
            (Path(directory) / (bad_key + ".json")).write_bytes(bad_raw)
            with self.assertRaises(ValueError):
                store.read(bad_key)
            with self.assertRaises(ValueError):
                store.ingest(invalid)


if __name__ == "__main__":
    unittest.main()
