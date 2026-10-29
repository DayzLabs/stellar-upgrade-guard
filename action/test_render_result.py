from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("render_result.py")


class RenderResultTests(unittest.TestCase):
    def run_renderer(self, report):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            json_path = root / "report.json"
            err_path = root / "stderr.txt"
            mode_path = root / "mode.txt"
            output_path = root / "output.txt"
            summary_path = root / "summary.md"
            json_path.write_text(json.dumps(report), encoding="utf-8")
            err_path.write_text("", encoding="utf-8")
            mode_path.write_text("local", encoding="utf-8")
            environment = os.environ | {
                "GITHUB_OUTPUT": str(output_path),
                "GITHUB_STEP_SUMMARY": str(summary_path),
            }
            result = subprocess.run(
                [sys.executable, str(SCRIPT), "1", str(json_path), str(err_path), str(mode_path)],
                capture_output=True,
                text=True,
                check=False,
                env=environment,
            )
            return result, summary_path.read_text(encoding="utf-8")

    def test_summary_includes_sanitized_and_bounded_findings(self):
        report = {
            "result": "breaking",
            "summary": {"breaking": 21, "unknown": 0, "warning": 0},
            "findings": [
                {"impact": "breaking", "message": "token=secret-value\nreturn type changed"},
                *[
                    {"impact": "warning", "message": "x" * 300}
                    for _ in range(20)
                ],
            ],
        }

        result, summary = self.run_renderer(report)

        self.assertEqual(result.returncode, 1)
        self.assertIn("### Findings", summary)
        self.assertIn("- BREAKING: token=<redacted> return type changed", summary)
        self.assertNotIn("secret-value", summary)
        self.assertIn("- ...and 1 more finding(s)", summary)
        self.assertLessEqual(max(map(len, summary.splitlines())), 255)


if __name__ == "__main__":
    unittest.main()
