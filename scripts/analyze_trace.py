#!/usr/bin/env python3
"""
BourbakiMesh Telemetry Analysis Tool.

Parses, aggregates, and diagnoses flight trace records from
artifacts/coordinator_trace.jsonl to provide rich metrics, latency distributions,
failure autopsies, Git commit hash provenance, session tracking, and proof synthesis summaries.
"""

from __future__ import annotations

import argparse
import json
import os
import statistics
import sys
from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple


@dataclass
class SolverTelemetry:
    tier: str
    nodes_explored: int
    depth_reached: int
    tier1_duration_us: int
    tier2_duration_us: int
    fallback_reason: Optional[str] = None


@dataclass
class SubmissionRecord:
    timestamp: str
    event_type: str
    worker_id: str
    task_id: str
    theorem_name: str
    term_ast: Any
    thinking_trace: str
    genrm_score: float
    wasm_latency_us: Optional[int]
    server_validation_latency_us: int
    failure_class: Optional[Dict[str, Any]]
    client_metadata: Optional[Dict[str, Any]]
    session_id: str = "legacy_session"
    server_commit: str = "legacy_commit"
    client_commit: Optional[str] = None
    solver_telemetry: Optional[SolverTelemetry] = None


def count_ast_nodes(expr: Any) -> int:
    """Recursively counts the number of AST nodes in a CIC or DeductionStep expression."""
    if not isinstance(expr, dict):
        return 1
    count = 1
    for _, v in expr.items():
        if isinstance(v, list):
            for item in v:
                count += count_ast_nodes(item)
        elif isinstance(v, dict):
            count += count_ast_nodes(v)
    return count


def format_cic_expr(expr: Any) -> str:
    """Formats a CIC AST expression into clean mathematical notation."""
    if not isinstance(expr, dict):
        return str(expr)
    if "Sort" in expr:
        return f"Sort({expr['Sort']})"
    if "BVar" in expr:
        return f"BVar({expr['BVar']})"
    if "FVar" in expr:
        return f"FVar({expr['FVar']})"
    if "Const" in expr:
        name = expr["Const"][0] if isinstance(expr["Const"], list) and expr["Const"] else str(expr["Const"])
        return name
    if "App" in expr:
        f, arg = expr["App"]
        return f"({format_cic_expr(f)} {format_cic_expr(arg)})"
    if "Lam" in expr:
        name, dom, body = expr["Lam"]
        return f"(λ {name} : {format_cic_expr(dom)} => {format_cic_expr(body)})"
    if "ForallE" in expr:
        name, dom, body = expr["ForallE"]
        return f"(∀ {name} : {format_cic_expr(dom)}, {format_cic_expr(body)})"
    return json.dumps(expr)


def parse_failure_class(fc: Any) -> Tuple[str, Dict[str, Any]]:
    """Normalizes the serialized FailureClass enum into a (class_name, details) tuple."""
    if isinstance(fc, str):
        return fc, {"message": fc}
    if isinstance(fc, dict):
        for k, v in fc.items():
            if isinstance(v, dict):
                return k, v
            elif isinstance(v, str):
                return k, {"message": v}
            else:
                return k, {"details": v}
    return "UnknownFailure", {"raw": fc}


def load_records(filepath: Path) -> List[SubmissionRecord]:
    """Ingests and parses all proof submission records from a coordinator JSONL trace."""
    if not filepath.exists():
        print(f"❌ Error: Trace file '{filepath}' not found.", file=sys.stderr)
        sys.exit(1)

    records: List[SubmissionRecord] = []

    with open(filepath, "r", encoding="utf-8") as f:
        for line_no, line in enumerate(f, 1):
            line = line.strip()
            if not line:
                continue
            try:
                data = json.loads(line)
            except json.JSONDecodeError:
                continue

            event_type = data.get("event_type", "")
            if event_type in ("PROOF_SUBMISSION_ACCEPTED", "PROOF_SUBMISSION_REJECTED"):
                client_meta = data.get("client_metadata")
                client_commit = data.get("client_commit")
                if not client_commit and isinstance(client_meta, dict):
                    client_commit = client_meta.get("client_commit")

                solver_tel_raw = data.get("solver_telemetry")
                solver_tel = None
                if isinstance(solver_tel_raw, dict):
                    solver_tel = SolverTelemetry(
                        tier=solver_tel_raw.get("tier", "unknown"),
                        nodes_explored=int(solver_tel_raw.get("nodes_explored", 1)),
                        depth_reached=int(solver_tel_raw.get("depth_reached", 0)),
                        tier1_duration_us=int(solver_tel_raw.get("tier1_duration_us", 0)),
                        tier2_duration_us=int(solver_tel_raw.get("tier2_duration_us", 0)),
                        fallback_reason=solver_tel_raw.get("fallback_reason"),
                    )

                records.append(
                    SubmissionRecord(
                        timestamp=data.get("timestamp", ""),
                        event_type=event_type,
                        worker_id=data.get("worker_id", "unknown_worker"),
                        task_id=data.get("task_id", "unknown_task"),
                        theorem_name=data.get("theorem_name", "unknown_theorem"),
                        term_ast=data.get("term_ast"),
                        thinking_trace=data.get("thinking_trace", ""),
                        genrm_score=float(data.get("genrm_score", 0.0)),
                        wasm_latency_us=data.get("wasm_latency_us"),
                        server_validation_latency_us=int(data.get("server_validation_latency_us", 0)),
                        failure_class=data.get("failure_class"),
                        client_metadata=client_meta,
                        session_id=data.get("session_id", "legacy_session"),
                        server_commit=data.get("server_commit", "legacy_commit"),
                        client_commit=client_commit,
                        solver_telemetry=solver_tel,
                    )
                )

    return records


def compute_latency_stats(latencies: List[int | float]) -> Dict[str, float]:
    """Computes mean, median, min, max, and p95 for a list of latency values in microseconds."""
    if not latencies:
        return {"count": 0, "mean": 0.0, "median": 0.0, "min": 0.0, "max": 0.0, "p95": 0.0}

    sorted_l = sorted(latencies)
    n = len(sorted_l)
    p95_idx = int(0.95 * (n - 1))

    return {
        "count": n,
        "mean": statistics.mean(sorted_l),
        "median": statistics.median(sorted_l),
        "min": float(sorted_l[0]),
        "max": float(sorted_l[-1]),
        "p95": float(sorted_l[p95_idx]),
    }


def analyze_trace(records: List[SubmissionRecord]) -> Dict[str, Any]:
    """Aggregates metrics, failure groupings, and acceptance statistics from submission records."""
    total = len(records)
    accepted = [r for r in records if r.event_type == "PROOF_SUBMISSION_ACCEPTED"]
    rejected = [r for r in records if r.event_type == "PROOF_SUBMISSION_REJECTED"]

    acceptance_rate = (len(accepted) / total * 100.0) if total > 0 else 0.0

    server_latencies = [r.server_validation_latency_us for r in records]
    wasm_latencies = [r.wasm_latency_us for r in records if r.wasm_latency_us is not None]

    # Failure breakdowns
    failures_by_class: Dict[str, List[SubmissionRecord]] = defaultdict(list)
    failures_by_theorem: Dict[str, List[SubmissionRecord]] = defaultdict(list)

    for r in rejected:
        fc_name, _ = parse_failure_class(r.failure_class)
        failures_by_class[fc_name].append(r)
        failures_by_theorem[r.theorem_name].append(r)

    # Accepted theorems
    accepted_by_theorem: Dict[str, Dict[str, Any]] = {}
    for r in accepted:
        ast_nodes = count_ast_nodes(r.term_ast)
        ast_bytes = len(json.dumps(r.term_ast)) if r.term_ast else 0
        accepted_by_theorem[r.theorem_name] = {
            "theorem_name": r.theorem_name,
            "task_id": r.task_id,
            "worker_id": r.worker_id,
            "server_validation_latency_us": r.server_validation_latency_us,
            "wasm_latency_us": r.wasm_latency_us,
            "genrm_score": r.genrm_score,
            "ast_nodes": ast_nodes,
            "ast_bytes": ast_bytes,
            "thinking_trace": r.thinking_trace,
            "term_ast": r.term_ast,
            "session_id": r.session_id,
            "server_commit": r.server_commit,
            "client_commit": r.client_commit,
        }

    # Distinct sessions and commits
    sessions = set(r.session_id for r in records)
    server_commits = set(r.server_commit for r in records)
    client_commits = set(r.client_commit for r in records if r.client_commit)

    # Solver Interplay Metrics
    solver_records = [r for r in records if r.solver_telemetry is not None]
    tier1_records = [r for r in solver_records if r.solver_telemetry.tier == "tier1_symbolic"]
    tier2_records = [r for r in solver_records if r.solver_telemetry.tier == "tier2_neural_search"]

    tier1_durations = [r.solver_telemetry.tier1_duration_us for r in tier1_records]
    tier2_durations = [r.solver_telemetry.tier2_duration_us for r in tier2_records]
    tier2_nodes = [r.solver_telemetry.nodes_explored for r in tier2_records]
    tier2_depths = [r.solver_telemetry.depth_reached for r in tier2_records]

    escalations = [
        {
            "theorem_name": r.theorem_name,
            "fallback_reason": r.solver_telemetry.fallback_reason or "unprovable_constructively",
            "nodes_explored": r.solver_telemetry.nodes_explored,
            "depth_reached": r.solver_telemetry.depth_reached,
            "tier1_duration_us": r.solver_telemetry.tier1_duration_us,
            "tier2_duration_us": r.solver_telemetry.tier2_duration_us,
            "accepted": r.event_type == "PROOF_SUBMISSION_ACCEPTED",
        }
        for r in tier2_records
    ]

    solver_interplay = {
        "total_tracked": len(solver_records),
        "tier1_count": len(tier1_records),
        "tier2_count": len(tier2_records),
        "tier1_pct": (len(tier1_records) / len(solver_records) * 100.0) if solver_records else 0.0,
        "tier2_pct": (len(tier2_records) / len(solver_records) * 100.0) if solver_records else 0.0,
        "tier1_duration_stats": compute_latency_stats(tier1_durations),
        "tier2_duration_stats": compute_latency_stats(tier2_durations),
        "tier2_mean_nodes": statistics.mean(tier2_nodes) if tier2_nodes else 0.0,
        "tier2_max_nodes": max(tier2_nodes) if tier2_nodes else 0,
        "tier2_mean_depth": statistics.mean(tier2_depths) if tier2_depths else 0.0,
        "tier2_max_depth": max(tier2_depths) if tier2_depths else 0,
        "escalations": escalations,
    }

    return {
        "total_submissions": total,
        "accepted_count": len(accepted),
        "rejected_count": len(rejected),
        "acceptance_rate_pct": acceptance_rate,
        "server_latency_stats": compute_latency_stats(server_latencies),
        "wasm_latency_stats": compute_latency_stats(wasm_latencies),
        "failures_by_class": {k: len(v) for k, v in failures_by_class.items()},
        "failures_by_theorem": {k: len(v) for k, v in failures_by_theorem.items()},
        "rejected_records": rejected,
        "accepted_theorems": accepted_by_theorem,
        "sessions_count": len(sessions),
        "server_commits": list(server_commits),
        "client_commits": list(client_commits),
        "solver_interplay": solver_interplay,
    }


def print_cli_report(analysis: Dict[str, Any], failures_only: bool = False, session_filter: Optional[str] = None, commit_filter: Optional[str] = None) -> None:
    """Prints a structured, formatted CLI telemetry analysis report."""
    total = analysis["total_submissions"]
    accepted = analysis["accepted_count"]
    rejected = analysis["rejected_count"]
    rate = analysis["acceptance_rate_pct"]

    if not failures_only:
        print("=" * 80)
        print("  ℬ BourbakiMesh Flight Recording & Telemetry Analysis")
        print("=" * 80)
        print()
        if session_filter:
            print(f"🎯 Session Filter:       {session_filter}")
        if commit_filter:
            print(f"📌 Commit Filter:        {commit_filter}")
        print(f"🌐 Sessions Detected:    {analysis['sessions_count']}")
        print(f"🏷️  Server Commits:       {', '.join(analysis['server_commits']) or 'None'}")
        print(f"📱 Client Commits:       {', '.join(analysis['client_commits']) or 'None'}")
        print()
        print("📊 OVERALL SUBMISSION METRICS")
        print("-" * 80)
        print(f"  • Total Submissions:      {total:,}")
        print(f"  • Accepted Proofs:        {accepted:,} ({rate:.1f}%)")
        print(f"  • Rejected Submissions:   {rejected:,} ({100.0 - rate:.1f}%)")
        print()

        # Latencies
        srv_stats = analysis["server_latency_stats"]
        wasm_stats = analysis["wasm_latency_stats"]

        print("⏱️  LATENCY BENCHMARKS (Microseconds µs)")
        print("-" * 80)
        print(f"  Server Kernel Typecheck:  Mean: {srv_stats['mean']:.1f}µs | Median: {srv_stats['median']:.1f}µs | Min: {srv_stats['min']:.0f}µs | Max: {srv_stats['max']:.0f}µs | P95: {srv_stats['p95']:.0f}µs")
        if wasm_stats["count"] > 0:
            print(f"  Client WASM Pre-Check:    Mean: {wasm_stats['mean']:.1f}µs | Median: {wasm_stats['median']:.1f}µs | Min: {wasm_stats['min']:.0f}µs | Max: {wasm_stats['max']:.0f}µs | P95: {wasm_stats['p95']:.0f}µs")
        print()

        # Accepted Summary
        accepted_thms = analysis["accepted_theorems"]
        print(f"🏆 ACCEPTED PROOF TARGETS ({len(accepted_thms)} Theorems Proven)")
        print("-" * 80)
        print(f"  {'Theorem Name':<28} | {'Server Latency':<15} | {'WASM Latency':<13} | {'AST Nodes':<10} | {'GenRM':<6}")
        print("  " + "-" * 78)
        for thm_name, info in sorted(accepted_thms.items()):
            wasm_str = f"{info['wasm_latency_us']}µs" if info['wasm_latency_us'] is not None else "N/A"
            print(f"  {thm_name:<28} | {info['server_validation_latency_us']:>6}µs         | {wasm_str:>10}  | {info['ast_nodes']:>6}     | {info['genrm_score']:.2f}")
        print()

        # Neuro-Symbolic Solver Interplay
        si = analysis.get("solver_interplay", {})
        if si.get("total_tracked", 0) > 0:
            print("🧠 NEURO-SYMBOLIC SOLVER INTERPLAY")
            print("-" * 80)
            print(f"  • Tracked Proof Attempts:           {si['total_tracked']:,}")
            print(f"  • Tier 1 (Symbolic Fast Path):      {si['tier1_count']:,} ({si['tier1_pct']:.1f}%)")
            print(f"  • Tier 2 (Neural Actor-Critic Search): {si['tier2_count']:,} ({si['tier2_pct']:.1f}%)")
            print()
            if si["tier1_count"] > 0:
                t1 = si["tier1_duration_stats"]
                print(f"  Tier 1 Duration:          Mean: {t1['mean']:.1f}µs | Median: {t1['median']:.1f}µs | Min: {t1['min']:.0f}µs | Max: {t1['max']:.0f}µs")
            if si["tier2_count"] > 0:
                t2 = si["tier2_duration_stats"]
                print(f"  Tier 2 Duration:          Mean: {t2['mean']:.1f}µs | Median: {t2['median']:.1f}µs | Min: {t2['min']:.0f}µs | Max: {t2['max']:.0f}µs")
                print(f"  Tier 2 Search Dynamics:   Nodes Explored: Mean {si['tier2_mean_nodes']:.1f} (Max {si['tier2_max_nodes']}) | Tree Depth: Mean {si['tier2_mean_depth']:.1f} (Max {si['tier2_max_depth']})")
                print()
                print("  Escalation Breakdown:")
                print(f"  {'Theorem Name':<24} | {'Fallback Reason':<28} | {'Nodes':<6} | {'Tier 1 (µs)':<12} | {'Tier 2 (µs)':<12}")
                print("  " + "-" * 90)
                for esc in si["escalations"]:
                    print(f"  {esc['theorem_name']:<24} | {esc['fallback_reason']:<28} | {esc['nodes_explored']:>5} | {esc['tier1_duration_us']:>10}µs | {esc['tier2_duration_us']:>10}µs")
            print()

    # Failure Autopsies
    rejected_records: List[SubmissionRecord] = analysis["rejected_records"]
    if rejected_records:
        print("⚠️  FAILURE BREAKDOWN & DIAGNOSTICS")
        print("-" * 80)
        fc_counts = analysis["failures_by_class"]
        for fc_name, count in fc_counts.items():
            print(f"  • {fc_name}: {count} occurrences")
        print()

        print("🔍 DETAILED FAILURE AUTOPSIES")
        print("-" * 80)
        for i, rec in enumerate(rejected_records, 1):
            fc_name, details = parse_failure_class(rec.failure_class)
            print(f"[{i}] Theorem: {rec.theorem_name} | Worker: {rec.worker_id} | Task: {rec.task_id}")
            print(f"    Session: {rec.session_id} | Commit: [S: {rec.server_commit} / C: {rec.client_commit or 'N/A'}]")
            print(f"    Timestamp: {rec.timestamp} | Server Validation: {rec.server_validation_latency_us}µs")
            print(f"    Failure Class: {fc_name}")

            if fc_name == "TypeMismatch":
                exp = details.get("expected", "Unknown")
                got = details.get("inferred", details.get("got", "Unknown"))
                print(f"    ┌─ Expected Type : {exp}")
                print(f"    └─ Inferred Type : {got}")
            elif "message" in details:
                print(f"    Message: {details['message']}")
            elif "details" in details:
                print(f"    Details: {details['details']}")

            if rec.thinking_trace:
                clean_think = " ".join(rec.thinking_trace.strip().split())
                if len(clean_think) > 120:
                    clean_think = clean_think[:117] + "..."
                print(f"    Thinking Trace : {clean_think}")

            if rec.term_ast:
                pretty_ast = format_cic_expr(rec.term_ast)
                if len(pretty_ast) > 120:
                    pretty_ast = pretty_ast[:117] + "..."
                print(f"    Faulty Term AST: {pretty_ast}")
            print()
    else:
        if failures_only:
            print("✅ No rejected proof submissions found in trace.")
        else:
            print("✅ Zero failures recorded! 100% acceptance across all attempts.")
            print()


def print_commit_comparison(records: List[SubmissionRecord]) -> None:
    """Displays a side-by-side comparative table of metrics grouped across all detected Git commits."""
    groups: Dict[str, List[SubmissionRecord]] = defaultdict(list)
    for r in records:
        key = f"{r.server_commit} (Client: {r.client_commit or 'N/A'})"
        groups[key].append(r)

    print("=" * 95)
    print("  ℬ BourbakiMesh Commit Provenance & Performance Comparative Matrix")
    print("=" * 95)
    print()
    print(f"  {'Commit Target':<36} | {'Attempts':<9} | {'Accepted':<14} | {'Server Mean':<13} | {'WASM Mean':<11}")
    print("  " + "-" * 91)

    for commit_key, recs in sorted(groups.items()):
        analysis = analyze_trace(recs)
        total = analysis["total_submissions"]
        acc = analysis["accepted_count"]
        rate = analysis["acceptance_rate_pct"]
        srv_mean = f"{analysis['server_latency_stats']['mean']:.1f}µs"
        wasm_mean = f"{analysis['wasm_latency_stats']['mean']:.1f}µs" if analysis['wasm_latency_stats']['count'] > 0 else "N/A"
        acc_str = f"{acc}/{total} ({rate:.1f}%)"

        print(f"  {commit_key:<36} | {total:>8}  | {acc_str:>14} | {srv_mean:>13} | {wasm_mean:>11}")
    print()


def main() -> None:
    parser = argparse.ArgumentParser(
        description="BourbakiMesh Flight Recording & Telemetry Analysis Engine"
    )
    parser.add_argument(
        "--file",
        type=str,
        default="artifacts/coordinator_trace.jsonl",
        help="Path to coordinator JSONL trace file (default: artifacts/coordinator_trace.jsonl)",
    )
    parser.add_argument(
        "--failures-only",
        action="store_true",
        help="Only display failure autopsies and error distributions",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Output machine-readable aggregated summary in JSON format",
    )
    parser.add_argument(
        "--commit",
        type=str,
        help="Filter metrics to a specific server or client commit hash prefix",
    )
    parser.add_argument(
        "--session",
        type=str,
        help="Filter metrics to a specific coordinator session ID",
    )
    parser.add_argument(
        "--latest-session",
        action="store_true",
        help="Automatically analyze only the most recent coordinator run session",
    )
    parser.add_argument(
        "--by-commit",
        action="store_true",
        help="Group and display a comparative matrix of acceptance rates and latency across Git commits",
    )

    args = parser.parse_args()
    trace_path = Path(args.file)

    records = load_records(trace_path)

    if not records:
        print(f"ℹ️  No proof submission records found in '{trace_path}'.")
        return

    if args.by_commit:
        print_commit_comparison(records)
        return

    active_session_filter = None
    if args.latest_session:
        latest_session_id = records[-1].session_id
        records = [r for r in records if r.session_id == latest_session_id]
        active_session_filter = f"{latest_session_id} (Latest)"

    if args.session:
        records = [r for r in records if r.session_id.startswith(args.session)]
        active_session_filter = args.session

    active_commit_filter = None
    if args.commit:
        records = [
            r for r in records
            if r.server_commit.startswith(args.commit) or (r.client_commit and r.client_commit.startswith(args.commit))
        ]
        active_commit_filter = args.commit

    analysis = analyze_trace(records)

    if args.json:
        json_output = {
            "file": str(trace_path),
            "session_filter": active_session_filter,
            "commit_filter": active_commit_filter,
            "total_submissions": analysis["total_submissions"],
            "accepted_count": analysis["accepted_count"],
            "rejected_count": analysis["rejected_count"],
            "acceptance_rate_pct": analysis["acceptance_rate_pct"],
            "server_latency_stats": analysis["server_latency_stats"],
            "wasm_latency_stats": analysis["wasm_latency_stats"],
            "solver_interplay": analysis["solver_interplay"],
            "failures_by_class": analysis["failures_by_class"],
            "failures_by_theorem": analysis["failures_by_theorem"],
            "accepted_theorems": analysis["accepted_theorems"],
            "sessions_count": analysis["sessions_count"],
            "server_commits": analysis["server_commits"],
            "client_commits": analysis["client_commits"],
        }
        print(json.dumps(json_output, indent=2))
    else:
        print_cli_report(
            analysis,
            failures_only=args.failures_only,
            session_filter=active_session_filter,
            commit_filter=active_commit_filter,
        )


if __name__ == "__main__":
    main()
