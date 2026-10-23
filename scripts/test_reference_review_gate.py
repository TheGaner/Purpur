#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Isolated regression tests for Phase -1 source-review digest drift detection."""
import contextlib
import hashlib
import io
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch
import reference_review_gate as review


class ReviewGateStalenessTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        (self.root / 'src').mkdir()
        (self.root / 'scripts').mkdir()
        (self.root / 'docs').mkdir()
        (self.root / 'docs' / 'audit.md').write_text('isolated fixture')
        self.source = self.root / 'src' / 'sample.rs'
        self.source.write_text('// initial contents\n')
        self.helper = self.root / 'scripts' / 'helper.py'
        self.helper.write_text('# test helper\n')
        paths = ['src/sample.rs', 'scripts/helper.py']
        self.review_row = {
            'id': 'REV-FAKE', 'status': 'in_review', 'paths': paths,
            'references': ['source implementation compared'],
            'evidence': 'docs/audit.md', 'approved_sha256': {},
            'current_sha256': {p: hashlib.sha256((self.root / p).read_bytes()).hexdigest() for p in paths},
        }
        self.write_ledger()

    def write_ledger(self):
        (self.root / 'docs' / 'ledger.json').write_text(json.dumps({
            'version': 1, 'reviews': [self.review_row],
        }))

    def check(self, enforce=False):
        fake = subprocess.CompletedProcess(args=[], returncode=0,
            stdout='src/sample.rs\nscripts/helper.py\n', stderr='')
        with patch.object(review, 'ROOT', self.root), \
             patch.object(review, 'MANIFEST', self.root / 'docs' / 'ledger.json'), \
             patch.object(review.subprocess, 'run', return_value=fake), \
             contextlib.redirect_stdout(io.StringIO()), \
             contextlib.redirect_stderr(io.StringIO()):
            return review.run(enforce)

    def test_valid_reviewed_digests_and_pending_unchanged(self):
        self.assertEqual(self.check(), 0)
        self.review_row['status'] = 'pending'
        self.review_row.pop('current_sha256')
        self.write_ledger()
        self.assertEqual(self.check(), 0)

    def test_stale_in_review_source_hash_fails_check(self):
        self.assertEqual(self.check(), 0)
        self.source.write_text('// changed after last review\n')
        self.assertEqual(self.check(), 1)

    def test_missing_in_review_source_digest_fails_check(self):
        self.review_row['current_sha256'].pop('scripts/helper.py')
        self.write_ledger()
        self.assertEqual(self.check(), 1)


if __name__ == '__main__':
    unittest.main()
