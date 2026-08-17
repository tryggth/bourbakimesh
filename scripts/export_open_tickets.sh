#!/usr/bin/env bash
set -euo pipefail

REPO="tryggth/bourbakimesh"
OUTPUT_FILE="reports/open_tickets_inventory.md"
mkdir -p reports

echo "# BourbakiMesh Open Tickets Inventory" > "$OUTPUT_FILE"
echo "**Generated:** $(date -u)" >> "$OUTPUT_FILE"
echo "**Repository:** https://github.com/${REPO}" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"

# Fetch open issues via GitHub REST API and format as markdown sections
gh api "repos/${REPO}/issues?state=open&per_page=100" --jq '
  .[] | select(.pull_request == null) |
  "## Issue #\(.number): \(.title)\n**URL:** \(.html_url)\n**Labels:** \([.labels[].name] | join(", "))\n\n### Current Body\n\(.body // "No description provided.")\n\n---\n"
' >> "$OUTPUT_FILE"

echo "✅ Exported open issues to ${OUTPUT_FILE}"
