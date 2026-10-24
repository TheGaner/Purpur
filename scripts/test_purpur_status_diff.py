#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Review-only Purpur first differential comparator unit validation."""
import unittest

from purpur_status_diff import (
    compare_status, endpoint, validate_status, varint,
)


def status(version_name="Minecraft 26.3", players_max=20):
    return {
        "version": {"name": version_name, "protocol": 777},
        "players": {"max": players_max, "online": 0},
        "description": {"text": "test"},
        "favicon": "data:image/png;base64,AAAA",
    }


class ComparatorTests(unittest.TestCase):
    def test_identical(self):
        actual = compare_status(status(), status())
        self.assertEqual(actual["differences"], [])
        self.assertEqual(actual["unapproved_differences"], 0)

    def test_purpur_default_vs_proto_is_not_silently_parity(self):
        actual = compare_status(status(),
                                status("Horizon-Purpur 26.3 prototype", 0))
        self.assertEqual(actual["unapproved_differences"], 2)
        self.assertEqual([x["field"] for x in actual["differences"]],
                         ["version.name", "players.max"])

    def test_approved_differences_are_still_reported(self):
        actual = compare_status(status(),
                                status("Horizon-Purpur 26.3 prototype", 0),
                                frozenset(["players.max"]))
        self.assertEqual(actual["unapproved_differences"], 1)
        self.assertEqual(len(actual["differences"]), 2)
        self.assertTrue(next(x for x in actual["differences"]
                             if x["field"] == "players.max")["accepted_deviation"])

    def test_wrong_type_boolean_not_integer(self):
        sample = status()
        sample["version"]["protocol"] = True
        with self.assertRaises(ValueError):
            validate_status(sample)

    def test_missing_required_fields(self):
        sample = status()
        del sample["description"]
        with self.assertRaises(ValueError):
            validate_status(sample)

    def test_optional_favicon_not_required(self):
        left, right = status(), status()
        del right["favicon"]
        actual = compare_status(left, right)
        self.assertEqual(actual["differences"], [])
        self.assertEqual(actual["reference_optional_fields"], ["favicon"])

    def test_endpoint_rejects_bad_ports(self):
        for value in ("", "localhost", ":25565", "localhost:0",
                      "localhost:65536", "localhost:no"):
            with self.subTest(value=value), self.assertRaises(ValueError):
                endpoint(value)

    def test_varint_length_boundaries(self):
        self.assertEqual(varint(0), b"\x00")
        self.assertEqual(varint(127), b"\x7f")
        self.assertEqual(varint(128), b"\x80\x01")


if __name__ == "__main__":
    unittest.main()
