"""CLI entrypoint for BourbakiMesh Python ML engine."""

import argparse
import sys
from bourbakimesh import __version__


def main():
    parser = argparse.ArgumentParser(
        description="BourbakiMesh ML & Latent MCTS Orchestration Engine"
    )
    parser.add_argument(
        "--version",
        action="version",
        version=f"bourbakimesh {__version__}",
    )
    parser.add_argument(
        "--serve",
        action="store_true",
        help="Start the FastAPI / gRPC orchestration API server",
    )
    parser.add_argument(
        "--port",
        type=int,
        default=8080,
        help="Port to listen on (default: 8080)",
    )
    args = parser.parse_args()

    if args.serve:
        import uvicorn
        from bourbakimesh.api.server import app

        print(f"Starting BourbakiMesh server on port {args.port}...")
        uvicorn.run(app, host="0.0.0.0", port=args.port)
    else:
        print(f"BourbakiMesh v{__version__} CLI ready. Run with --help for options.")


if __name__ == "__main__":
    main()
