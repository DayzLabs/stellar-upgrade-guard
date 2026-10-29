#!/usr/bin/env python3
"""Offline regression tests for the composite action's result renderer."""

import json
import os
from contextlib import redirect_stderr, redirect_stdout
import io
from pathlib import Path
import sys
import tempfile
import unittest


RENDER_RESULT = Path(__file__).with_name("render_result.py")
sys.path.insert(0, str(RENDER_RESULT.parent))
from render_result import main  # noqa: E402


class RenderResultTests(unittest.TestCase):
    def run_renderer(self, status, report, stderr="", mode="local"):
        with tempfile.TemporaryDirectory(dir=RENDER_RESULT.parent) as directory:
            root = Path(directory)
            json_path = root / "report.json"
            err_path = root / "stderr.txt"
            mode_path = root / "mode.txt"
            output_path = root / "github-output.txt"
            summary_path = root / "github-summary.md"

            json_path.write_text(report, encoding="utf-8")
            err_path.write_text(stderr, encoding="utf-8")
            mode_path.write_text(mode, encoding="utf-8")

            old_argv = sys.argv
            old_environment = {
                name: os.environ.get(name)
                for name in ("GITHUB_OUTPUT", "GITHUB_STEP_SUMMARY")
            }
            stdout = io.StringIO()
            captured_stderr = io.StringIO()
            try:
                os.environ["GITHUB_OUTPUT"] = str(output_path)
                os.environ["GITHUB_STEP_SUMMARY"] = str(summary_path)
                sys.argv = [
                    str(RENDER_RESULT),
                    str(status),
                    str(json_path),
                    str(err_path),
                    str(mode_path),
                ]
                with redirect_stdout(stdout), redirect_stderr(captured_stderr):
                    returncode = main()
            finally:
                sys.argv = old_argv
                for name, value in old_environment.items():
                    if value is None:
                        os.environ.pop(name, None)
                    else:
                        os.environ[name] = value

            output = output_path.read_text(encoding="utf-8") if output_path.exists() else None
            summary = summary_path.read_text(encoding="utf-8") if summary_path.exists() else None
            return returncode, stdout.getvalue(), captured_stderr.getvalue(), output, summary

    def test_valid_report_writes_outputs_and_summary(self):
        report = json.dumps(
            {
                "result": "compatible",
                "summary": {"breaking": 0, "unknown": 1, "warning": 2},
                "findings": [{"impact": "warning", "message": "review this change"}],
            }
        )

        returncode, stdout, _, outputs, summary = self.run_renderer(
            0, report, mode="deployed"
        )

        self.assertEqual(returncode, 0)
        self.assertIn("Result: COMPATIBLE", stdout)
        self.assertIn("- WARNING: review this change", stdout)
        self.assertIsNotNone(outputs)
        self.assertEqual(
            outputs.splitlines(),
            [
                "result=compatible",
                "breaking-count=0",
                "unknown-count=1",
                "warning-count=2",
            ],
        )
        self.assertIsNotNone(summary)
        self.assertIn("Mode: deployed", summary)
        self.assertIn("Warnings: 2", summary)

    def test_malformed_json_returns_parse_error_and_redacts_stderr(self):
        secrets = (
            "Authorization: Bearer authorization-secret "
            "api-key=api-secret token=token-secret "
            "https://user:password@example.test/report?secret=value"
        )

        returncode, stdout, captured_stderr, outputs, summary = self.run_renderer(
            2, "{not-json", stderr=secrets
        )

        self.assertEqual(returncode, 2)
        self.assertIn("failed to parse stellar-upgrade-guard JSON output", captured_stderr)
        self.assertIn("<redacted>", captured_stderr)
        for secret in ("authorization-secret", "api-secret", "token-secret", "password", "secret=value"):
            self.assertNotIn(secret, captured_stderr)
        self.assertIsNone(outputs)
        self.assertIsNone(summary)

    def test_empty_json_preserves_status_and_sanitizes_stderr(self):
        returncode, stdout, captured_stderr, outputs, summary = self.run_renderer(
            1,
            "",
            stderr="comparison failed token=hidden-token",
        )

        self.assertEqual(returncode, 1)
        self.assertEqual(stdout, "")
        self.assertIn("comparison failed token=<redacted>", captured_stderr)
        self.assertNotIn("hidden-token", captured_stderr)
        self.assertIsNone(outputs)
        self.assertIsNone(summary)


if __name__ == "__main__":
    unittest.main()
