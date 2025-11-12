#!/usr/bin/env python3
"""
Generate ADR graph files (nodes.jsonl, edges.jsonl) from the ADR index.
"""

import json
import sys
import argparse
from pathlib import Path
from datetime import datetime

def parse_args():
    """Parse command line arguments."""
    parser = argparse.ArgumentParser(description="Generate ADR graph files")
    parser.add_argument(
        "--root",
        type=str,
        default=None,
        help="Root directory for ADRs (default: auto-detect from script location)"
    )
    parser.add_argument(
        "--index",
        type=str,
        default=None,
        help="Path to index file (default: docs/adr/adr.index.json)"
    )
    return parser.parse_args()

# Auto-detect repo root and ADR directory
SCRIPT_DIR = Path(__file__).parent
REPO_ROOT = SCRIPT_DIR.parent.parent

def generate_graph(adr_dir: Path, index_file: Path = None):
    """Generate graph files from index."""
    if index_file is None:
        index_file = adr_dir / "adr.index.json"
    
    if not index_file.exists():
        print(f"Error: Index file not found: {index_file}")
        return False
    
    print("Generating ADR graph files...")
    
    # Load index
    with open(index_file, 'r', encoding='utf-8') as f:
        index = json.load(f)
    
    nodes_file = adr_dir / "graph.nodes.jsonl"
    edges_file = adr_dir / "graph.edges.jsonl"
    
    nodes = []
    edges = []
    
    # Process each ADR
    for adr_entry in index.get("adrs", []):
        adr_id = adr_entry.get("adr_id", "")
        adr_num = adr_id.replace("ADR-", "") if adr_id.startswith("ADR-") else ""
        
        # Create ADR node
        adr_node = {
            "id": f"adr:{adr_num}",
            "type": "ADR",
            "adr_id": adr_id,
            "title": adr_entry.get("title", ""),
            "status": adr_entry.get("status", "unknown"),
            "created_at": adr_entry.get("created_at", "")
        }
        
        if adr_entry.get("accepted_at"):
            adr_node["accepted_at"] = adr_entry["accepted_at"]
        if adr_entry.get("implemented_at"):
            adr_node["implemented_at"] = adr_entry["implemented_at"]
        
        # Add text content (summary/rationale) - would need to read from ADR file
        # For now, we'll leave it empty or add a placeholder
        nodes.append(adr_node)
        
        # Process phases
        for phase in adr_entry.get("phases", []):
            phase_id = phase.get("phase_id", "")
            phase_seq = phase.get("sequence", 0)
            
            # Create PHASE node
            phase_node = {
                "id": f"phase:{adr_num}:P{phase_seq:02d}",
                "type": "PHASE",
                "phase_id": phase_id,
                "status": phase.get("status", "unknown"),
                "title": phase.get("title", "")  # Would need to read from phase file
            }
            
            if phase.get("implemented_at"):
                phase_node["implemented_at"] = phase["implemented_at"]
            
            nodes.append(phase_node)
            
            # Create HAS_PHASE edge
            edges.append({
                "src": f"adr:{adr_num}",
                "dst": f"phase:{adr_num}:P{phase_seq:02d}",
                "type": "HAS_PHASE"
            })
            
            # Create IMPLEMENTS_IN edge if phase is implemented
            if phase.get("status") == "implemented" and phase.get("implementation_commit"):
                commit_sha = phase["implementation_commit"]
                commit_node_id = f"commit:{commit_sha[:7]}"
                
                # Create COMMIT node (if not already created)
                commit_node = {
                    "id": commit_node_id,
                    "type": "COMMIT",
                    "sha": commit_sha
                }
                
                # Check if commit node already exists
                if not any(n.get("id") == commit_node_id for n in nodes):
                    nodes.append(commit_node)
                
                # Create IMPLEMENTS_IN edge
                edges.append({
                    "src": f"phase:{adr_num}:P{phase_seq:02d}",
                    "dst": commit_node_id,
                    "type": "IMPLEMENTS_IN"
                })
        
        # Create DEPENDS_ON edges
        for dep_adr_id in adr_entry.get("depends_on", []):
            dep_num = dep_adr_id.replace("ADR-", "") if dep_adr_id.startswith("ADR-") else ""
            edges.append({
                "src": f"adr:{adr_num}",
                "dst": f"adr:{dep_num}",
                "type": "DEPENDS_ON"
            })
    
    # Write nodes file (JSONL)
    with open(nodes_file, 'w', encoding='utf-8') as f:
        for node in nodes:
            f.write(json.dumps(node, ensure_ascii=False) + '\n')
    
    print(f"✓ Generated {len(nodes)} nodes: {nodes_file}")
    
    # Write edges file (JSONL)
    with open(edges_file, 'w', encoding='utf-8') as f:
        for edge in edges:
            f.write(json.dumps(edge, ensure_ascii=False) + '\n')
    
    print(f"✓ Generated {len(edges)} edges: {edges_file}")
    
    return True

if __name__ == "__main__":
    args = parse_args()
    if args.root:
        adr_dir = Path(args.root)
    else:
        adr_dir = REPO_ROOT / "docs" / "adr"
    
    index_file = Path(args.index) if args.index else None
    
    success = generate_graph(adr_dir, index_file)
    sys.exit(0 if success else 1)

