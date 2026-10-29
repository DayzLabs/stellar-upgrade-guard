#!/usr/bin/env python3
import json
import os
import re
import sys


MAX_SUMMARY_FINDINGS = 20
MAX_SUMMARY_MESSAGE_LENGTH = 240


def sanitize(text):
    text = re.sub(r"(?i)(authorization:\s*)\S+", r"\1<redacted>", text)
    text = re.sub(r"(?i)(api[-_]?key=)[^&\s]+", r"\1<redacted>", text)
    text = re.sub(r"(?i)(token=)[^&\s]+", r"\1<redacted>", text)
    text = re.sub(r"(https?://[^:/\s]+):[^@\s]+@", r"\1:<redacted>@", text)
    text = re.sub(r"(https?://[^\s?]+)\?[^)\]\s]+", r"\1?<redacted>", text)
    return text


def summary_finding_lines(findings):
    lines = []
    for finding in findings[:MAX_SUMMARY_FINDINGS]:
        impact = str(finding.get("impact", "unknown")).upper()
        message = " ".join(sanitize(str(finding.get("message", ""))).split())
        if len(message) > MAX_SUMMARY_MESSAGE_LENGTH:
            message = f"{message[: MAX_SUMMARY_MESSAGE_LENGTH - 3]}..."
        lines.append(f"- {impact}: {message}")

    remaining = len(findings) - len(lines)
    if remaining:
        lines.append(f"- ...and {remaining} more finding(s)")
    return lines


def write_output(name, value):
    output = os.environ.get("GITHUB_OUTPUT")
    if output:
        with open(output, "a", encoding="utf-8") as handle:
            handle.write(f"{name}={value}\n")


def append_summary(lines):
    summary = os.environ.get("GITHUB_STEP_SUMMARY")
    if summary:
        with open(summary, "a", encoding="utf-8") as handle:
            handle.write("\n".join(lines))
            handle.write("\n")


def main():
    status = int(sys.argv[1])
    json_path = sys.argv[2]
    err_path = sys.argv[3]
    mode_path = sys.argv[4]

    stderr = ""
    if os.path.exists(err_path):
        with open(err_path, encoding="utf-8") as handle:
            stderr = sanitize(handle.read().strip())

    report = None
    try:
        with open(json_path, encoding="utf-8") as handle:
            content = handle.read().strip()
        if content:
            report = json.loads(content)
    except json.JSONDecodeError as err:
        if stderr:
            print(stderr, file=sys.stderr)
        print(f"error: failed to parse stellar-upgrade-guard JSON output: {err}", file=sys.stderr)
        return 2

    if report is None:
        if stderr:
            print(stderr, file=sys.stderr)
        return status

    result = str(report.get("result", "unknown"))
    summary = report.get("summary", {})
    breaking = int(summary.get("breaking", 0))
    unknown = int(summary.get("unknown", 0))
    warning = int(summary.get("warning", 0))

    write_output("result", result)
    write_output("breaking-count", breaking)
    write_output("unknown-count", unknown)
    write_output("warning-count", warning)

    print("Stellar Upgrade Guard")
    print()
    print(f"Result: {result.upper()}")
    print(f"Breaking: {breaking}")
    print(f"Unknown: {unknown}")
    print(f"Warnings: {warning}")

    findings = report.get("findings", [])
    if findings:
        print()
        print("Findings:")
        for finding in findings:
            impact = str(finding.get("impact", "unknown")).upper()
            message = finding.get("message", "")
            print(f"- {impact}: {message}")

    mode = "unknown"
    if os.path.exists(mode_path):
        with open(mode_path, encoding="utf-8") as handle:
            mode = handle.read().strip()

    summary_lines = [
        "## Stellar Upgrade Guard",
        "",
        f"Mode: {mode}",
        f"Result: {result.upper()}",
        "",
        f"Breaking: {breaking}",
        f"Unknown: {unknown}",
        f"Warnings: {warning}",
    ]
    if findings:
        summary_lines.extend(["", "### Findings", ""])
        summary_lines.extend(summary_finding_lines(findings))
    append_summary(summary_lines)

    if stderr:
        print()
        print(stderr, file=sys.stderr)

    return status


if __name__ == "__main__":
    raise SystemExit(main())
