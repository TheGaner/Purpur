#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Offline unit tests for the dependency license snapshot drift checker."""
import copy
import unittest
import check_cargo_licenses as audit


class LicenseSnapshotRegression(unittest.TestCase):
    def setUp(self):
        self.original = {
            "schemaVersion": 1,
            "scope": "locked Cargo registry crates only; metadata declarations, not legal clearance",
            "packages": [
                {"name": "demo", "version": "1.0", "source": "registry+test",
                 "checksum": "a" * 64, "declaredLicense": "MIT", "repository": None}
            ],
        }

    def test_exact_snapshot_accepts(self):
        self.assertEqual(audit.differences(self.original, copy.deepcopy(self.original)), [])

    def test_license_or_checksum_drift_fails(self):
        for field, changed in [("declaredLicense", "GPL-3.0-only"),
                               ("checksum", "b" * 64)]:
            candidate = copy.deepcopy(self.original)
            candidate["packages"][0][field] = changed
            self.assertTrue(audit.differences(self.original, candidate))

    def test_added_removed_and_schema_change_fail(self):
        add = copy.deepcopy(self.original)
        add["packages"].append({
            **add["packages"][0], "name": "other", "checksum": "b" * 64,
        })
        self.assertTrue(audit.differences(self.original, add))
        remove = copy.deepcopy(self.original)
        remove["packages"].clear()
        self.assertTrue(audit.differences(self.original, remove))
        schema = copy.deepcopy(self.original)
        schema["schemaVersion"] = 2
        self.assertTrue(audit.differences(self.original, schema))


if __name__ == "__main__":
    unittest.main()
