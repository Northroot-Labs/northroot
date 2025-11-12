#!/usr/bin/env python3
"""
Linker tool: Maps commits → files → symbols and updates graph files.
Runs after make adr.index to populate code references and graph edges.
"""

import json
import sys
import subprocess
import argparse
from pathlib import Path
from collections import defaultdict
from datetime import datetime

def parse_args():
    """Parse command line arguments."""
    parser = argparse.ArgumentParser(description="Link commits to files and symbols")
    parser.add_argument(
        "--root",
        type=str,
        default=None,
        help="Root directory for ADRs (default: auto-detect)"
    )
    parser.add_argument(
        "--index",
        type=str,
        default=None,
        help="Path to index file (default: docs/adr/adr.index.json)"
    )
    parser.add_argument(
        "--no-symbols",
        action="store_true",
        help="Skip symbol extraction (file-level only)"
    )
    return parser.parse_args()

# Auto-detect repo root
SCRIPT_DIR = Path(__file__).parent
REPO_ROOT = SCRIPT_DIR.parent.parent

def get_commit_info(commit_sha: str, repo_root: Path) -> dict:
    """Get commit information from git."""
    try:
        # Get commit message
        result = subprocess.run(
            ["git", "log", "-1", "--format=%s", commit_sha],
            cwd=repo_root,
            capture_output=True,
            text=True,
            check=False
        )
        message = result.stdout.strip() if result.returncode == 0 else ""
        
        # Get commit timestamp
        result = subprocess.run(
            ["git", "log", "-1", "--format=%aI", commit_sha],
            cwd=repo_root,
            capture_output=True,
            text=True,
            check=False
        )
        when = result.stdout.strip() if result.returncode == 0 else ""
        
        return {
            "message": message,
            "when": when
        }
    except Exception as e:
        print(f"Warning: Failed to get commit info for {commit_sha}: {e}")
        return {"message": "", "when": ""}

def get_files_changed(commit_sha: str, repo_root: Path) -> list[str]:
    """Get list of files changed in a commit."""
    try:
        result = subprocess.run(
            ["git", "diff-tree", "--no-commit-id", "--name-only", "-r", commit_sha],
            cwd=repo_root,
            capture_output=True,
            text=True,
            check=False
        )
        if result.returncode == 0:
            return [f.strip() for f in result.stdout.strip().split('\n') if f.strip()]
    except Exception as e:
        print(f"Warning: Failed to get files for commit {commit_sha}: {e}")
    return []

def extract_symbols_from_file(file_path: Path, repo_root: Path, no_symbols: bool = False) -> list[dict]:
    """Extract symbols from a file using ctags or rust-analyzer."""
    if no_symbols:
        return []
    
    symbols = []
    full_path = repo_root / file_path
    
    if not full_path.exists():
        return []
    
    # Try ctags first (universal)
    try:
        result = subprocess.run(
            ["ctags", "-x", "--c-kinds=fp", str(full_path)],
            cwd=repo_root,
            capture_output=True,
            text=True,
            check=False
        )
        if result.returncode == 0:
            for line in result.stdout.strip().split('\n'):
                if not line.strip():
                    continue
                parts = line.split()
                if len(parts) >= 4:
                    symbol_name = parts[0]
                    line_num = int(parts[2]) if parts[2].isdigit() else 0
                    symbols.append({
                        "name": symbol_name,
                        "line": line_num
                    })
    except FileNotFoundError:
        # ctags not available, try rust-analyzer for Rust files
        if file_path.suffix == ".rs":
            try:
                # Use rust-analyzer via LSP or fallback to simple parsing
                # For now, we'll skip symbol extraction if ctags is not available
                pass
            except Exception:
                pass
    
    return symbols

def update_graph_files(adr_dir: Path, index_file: Path, no_symbols: bool = False):
    """Update graph files with commit → file → symbol mappings."""
    print("Linking commits to files and symbols...")
    
    # Load index
    with open(index_file, 'r', encoding='utf-8') as f:
        index = json.load(f)
    
    nodes_file = adr_dir / "graph.nodes.jsonl"
    edges_file = adr_dir / "graph.edges.jsonl"
    
    # Load existing nodes and edges
    nodes = {}
    edges = []
    
    if nodes_file.exists():
        with open(nodes_file, 'r', encoding='utf-8') as f:
            for line in f:
                if line.strip():
                    node = json.loads(line)
                    nodes[node["id"]] = node
    
    if edges_file.exists():
        with open(edges_file, 'r', encoding='utf-8') as f:
            for line in f:
                if line.strip():
                    edges.append(json.loads(line))
    
    # Track commits we've processed
    processed_commits = set()
    code_refs_by_adr = defaultdict(list)
    
    # Process each ADR and phase
    for adr_entry in index.get("adrs", []):
        adr_id = adr_entry.get("adr_id", "")
        adr_num = adr_id.replace("ADR-", "") if adr_id.startswith("ADR-") else ""
        
        for phase in adr_entry.get("phases", []):
            if phase.get("status") != "implemented":
                continue
            
            commit_sha = phase.get("implementation_commit")
            if not commit_sha or commit_sha in processed_commits:
                continue
            
            processed_commits.add(commit_sha)
            commit_node_id = f"commit:{commit_sha[:7]}"
            
            # Get commit info
            commit_info = get_commit_info(commit_sha, REPO_ROOT)
            
            # Update or create COMMIT node
            if commit_node_id not in nodes:
                nodes[commit_node_id] = {
                    "id": commit_node_id,
                    "type": "COMMIT",
                    "sha": commit_sha,
                    "message": commit_info["message"],
                    "when": commit_info["when"]
                }
            else:
                # Update existing node
                nodes[commit_node_id]["message"] = commit_info["message"]
                nodes[commit_node_id]["when"] = commit_info["when"]
            
            # Get files changed in this commit
            files_changed = get_files_changed(commit_sha, REPO_ROOT)
            
            for file_path_str in files_changed:
                # Skip ADR files themselves
                if file_path_str.startswith("docs/adr/"):
                    continue
                
                file_path = Path(file_path_str)
                file_node_id = f"file:{file_path_str}"
                
                # Create FILE node
                if file_node_id not in nodes:
                    nodes[file_node_id] = {
                        "id": file_node_id,
                        "type": "FILE",
                        "path": file_path_str
                    }
                
                # Create TOUCHES_FILE edge
                edge = {
                    "src": commit_node_id,
                    "dst": file_node_id,
                    "type": "TOUCHES_FILE"
                }
                if edge not in edges:
                    edges.append(edge)
                
                # Add code reference
                code_refs_by_adr[adr_id].append({
                    "type": "file",
                    "path": file_path_str,
                    "commit": commit_sha
                })
                
                # Extract symbols if enabled
                if not no_symbols:
                    symbols = extract_symbols_from_file(file_path, REPO_ROOT, no_symbols)
                    for symbol in symbols:
                        symbol_name = symbol["name"]
                        symbol_node_id = f"sym:{symbol_name}"
                        
                        # Create SYMBOL node
                        if symbol_node_id not in nodes:
                            nodes[symbol_node_id] = {
                                "id": symbol_node_id,
                                "type": "SYMBOL",
                                "path": file_path_str,
                                "symbol": symbol_name
                            }
                        
                        # Create TOUCHES_SYMBOL edge
                        edge = {
                            "src": commit_node_id,
                            "dst": symbol_node_id,
                            "type": "TOUCHES_SYMBOL"
                        }
                        if edge not in edges:
                            edges.append(edge)
                        
                        # Add code reference for symbol
                        code_refs_by_adr[adr_id].append({
                            "type": "symbol",
                            "path": file_path_str,
                            "commit": commit_sha,
                            "symbol": symbol_name,
                            "lines": [symbol["line"]] if symbol.get("line") else None
                        })
    
    # Update index with code_refs
    for adr_entry in index.get("adrs", []):
        adr_id = adr_entry.get("adr_id", "")
        if adr_id in code_refs_by_adr:
            # Merge with existing code_refs (avoid duplicates)
            existing_refs = {json.dumps(ref, sort_keys=True) for ref in adr_entry.get("code_refs", [])}
            new_refs = code_refs_by_adr[adr_id]
            for ref in new_refs:
                ref_key = json.dumps(ref, sort_keys=True)
                if ref_key not in existing_refs:
                    adr_entry.setdefault("code_refs", []).append(ref)
                    existing_refs.add(ref_key)
    
    # Write updated index
    with open(index_file, 'w', encoding='utf-8') as f:
        json.dump(index, f, indent=2)
    
    # Write updated nodes file
    with open(nodes_file, 'w', encoding='utf-8') as f:
        for node_id in sorted(nodes.keys()):
            f.write(json.dumps(nodes[node_id], ensure_ascii=False) + '\n')
    
    print(f"✓ Updated {len(nodes)} nodes: {nodes_file}")
    
    # Write updated edges file
    with open(edges_file, 'w', encoding='utf-8') as f:
        for edge in edges:
            f.write(json.dumps(edge, ensure_ascii=False) + '\n')
    
    print(f"✓ Updated {len(edges)} edges: {edges_file}")
    
    return True

if __name__ == "__main__":
    args = parse_args()
    if args.root:
        adr_dir = Path(args.root)
    else:
        adr_dir = REPO_ROOT / "docs" / "adr"
    
    index_file = Path(args.index) if args.index else (adr_dir / "adr.index.json")
    
    success = update_graph_files(adr_dir, index_file, args.no_symbols)
    sys.exit(0 if success else 1)

