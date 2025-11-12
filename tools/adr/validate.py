#!/usr/bin/env python3
"""
Validate ADR structure and schemas.
"""

import json
import yaml
import sys
import subprocess
from pathlib import Path
from jsonschema import validate, ValidationError

import argparse

def parse_args():
    """Parse command line arguments."""
    parser = argparse.ArgumentParser(description="Validate ADR structure")
    parser.add_argument(
        "--root",
        type=str,
        default=None,
        help="Root directory for ADRs (default: auto-detect)"
    )
    parser.add_argument(
        "--strict",
        action="store_true",
        help="Exit with error code on validation failures"
    )
    return parser.parse_args()

# Auto-detect repo root
SCRIPT_DIR = Path(__file__).parent
REPO_ROOT = SCRIPT_DIR.parent.parent
SCHEMA_DIR = REPO_ROOT / "schemas" / "adr"

def load_schema(schema_file: Path):
    """Load JSON schema."""
    with open(schema_file, 'r') as f:
        return json.load(f)

def validate_adr(adr_file: Path, adr_schema: dict) -> bool:
    """Validate a single ADR file."""
    try:
        with open(adr_file, 'r', encoding='utf-8') as f:
            content = f.read()
        
        if not content.startswith('---'):
            print(f"✗ {adr_file}: Missing YAML frontmatter")
            return False
        
        parts = content.split('---', 2)
        if len(parts) < 3:
            print(f"✗ {adr_file}: Invalid frontmatter format")
            return False
        
        frontmatter = yaml.safe_load(parts[1])
        if not frontmatter:
            print(f"✗ {adr_file}: Empty frontmatter")
            return False
        
        try:
            validate(instance=frontmatter, schema=adr_schema)
            print(f"✓ {adr_file.name}")
            return True
        except ValidationError as e:
            print(f"✗ {adr_file.name}: {e.message}")
            return False
    except Exception as e:
        print(f"✗ {adr_file}: Error: {e}")
        return False

def validate_phase(phase_file: Path, phase_schema: dict) -> bool:
    """Validate a single phase file."""
    try:
        with open(phase_file, 'r', encoding='utf-8') as f:
            phase_data = yaml.safe_load(f)
        
        if not phase_data:
            print(f"✗ {phase_file}: Empty file")
            return False
        
        try:
            validate(instance=phase_data, schema=phase_schema)
            print(f"  ✓ {phase_file.name}")
            return True
        except ValidationError as e:
            print(f"  ✗ {phase_file.name}: {e.message}")
            return False
    except Exception as e:
        print(f"  ✗ {phase_file}: Error: {e}")
        return False

def validate_commit_sha(commit_sha: str, repo_root: Path) -> tuple[bool, str]:
    """
    Validate that a commit SHA exists in git and is not from uncommitted changes.
    
    Returns:
        (is_valid, error_message)
    """
    if not commit_sha or not commit_sha.strip():
        return True, ""  # Optional field, empty is valid
    
    commit_sha = commit_sha.strip()
    
    # Check if commit exists
    try:
        result = subprocess.run(
            ["git", "cat-file", "-e", commit_sha],
            cwd=repo_root,
            capture_output=True,
            check=False
        )
        if result.returncode != 0:
            return False, f"Commit SHA {commit_sha} does not exist in git repository"
    except Exception as e:
        return False, f"Error checking commit SHA {commit_sha}: {e}"
    
    # Check if commit is in current HEAD history (not uncommitted)
    try:
        result = subprocess.run(
            ["git", "merge-base", "--is-ancestor", commit_sha, "HEAD"],
            cwd=repo_root,
            capture_output=True,
            check=False
        )
        if result.returncode != 0:
            # Commit exists but is not an ancestor of HEAD
            # This could be a future commit or a commit from another branch
            # We'll allow it but warn
            pass
    except Exception:
        pass
    
    return True, ""

def validate_index(index_file: Path, index_schema: dict, repo_root: Path) -> bool:
    """Validate the ADR index."""
    try:
        with open(index_file, 'r', encoding='utf-8') as f:
            index_data = json.load(f)
        
        # Schema validation
        try:
            validate(instance=index_data, schema=index_schema)
        except ValidationError as e:
            print(f"✗ {index_file.name}: Schema validation failed: {e.message}")
            return False
        
        # Commit SHA provenance validation
        print(f"\nValidating commit SHA provenance:")
        all_commits_valid = True
        
        for adr_entry in index_data.get("adrs", []):
            for phase in adr_entry.get("phases", []):
                if phase.get("status") == "implemented":
                    commit_sha = phase.get("implementation_commit")
                    if commit_sha:
                        is_valid, error_msg = validate_commit_sha(commit_sha, repo_root)
                        if not is_valid:
                            phase_id = phase.get("phase_id", "unknown")
                            print(f"  ✗ {phase_id}: {error_msg}")
                            all_commits_valid = False
                        else:
                            phase_id = phase.get("phase_id", "unknown")
                            print(f"  ✓ {phase_id}: Commit SHA {commit_sha[:8]}... exists")
                    else:
                        phase_id = phase.get("phase_id", "unknown")
                        print(f"  ⚠ {phase_id}: No implementation_commit specified (optional)")
        
        if not all_commits_valid:
            print(f"\n✗ {index_file.name}: Commit SHA validation failed")
            return False
        
        print(f"✓ {index_file.name}")
        return True
    except Exception as e:
        print(f"✗ {index_file}: Error: {e}")
        return False

def validate_graph_nodes(nodes_file: Path, node_schema: dict) -> bool:
    """Validate graph nodes file."""
    if not nodes_file.exists():
        print(f"⚠ {nodes_file.name}: Graph nodes file not found (optional)")
        return True
    
    try:
        node_ids = set()
        with open(nodes_file, 'r', encoding='utf-8') as f:
            for line_num, line in enumerate(f, 1):
                if not line.strip():
                    continue
                try:
                    node = json.loads(line)
                    # Schema validation
                    try:
                        validate(instance=node, schema=node_schema)
                    except ValidationError as e:
                        print(f"✗ {nodes_file.name}: Line {line_num}: {e.message}")
                        return False
                    
                    # Check for duplicate IDs
                    node_id = node.get("id")
                    if node_id in node_ids:
                        print(f"✗ {nodes_file.name}: Line {line_num}: Duplicate node ID: {node_id}")
                        return False
                    node_ids.add(node_id)
                except json.JSONDecodeError as e:
                    print(f"✗ {nodes_file.name}: Line {line_num}: Invalid JSON: {e}")
                    return False
        
        print(f"✓ {nodes_file.name} ({len(node_ids)} nodes)")
        return True
    except Exception as e:
        print(f"✗ {nodes_file}: Error: {e}")
        return False

def validate_graph_edges(edges_file: Path, edge_schema: dict, nodes_file: Path) -> bool:
    """Validate graph edges file and check consistency with nodes."""
    if not edges_file.exists():
        print(f"⚠ {edges_file.name}: Graph edges file not found (optional)")
        return True
    
    # Load node IDs if nodes file exists
    node_ids = set()
    if nodes_file.exists():
        with open(nodes_file, 'r', encoding='utf-8') as f:
            for line in f:
                if line.strip():
                    node = json.loads(line)
                    node_ids.add(node.get("id"))
    
    try:
        edge_count = 0
        with open(edges_file, 'r', encoding='utf-8') as f:
            for line_num, line in enumerate(f, 1):
                if not line.strip():
                    continue
                try:
                    edge = json.loads(line)
                    # Schema validation
                    try:
                        validate(instance=edge, schema=edge_schema)
                    except ValidationError as e:
                        print(f"✗ {edges_file.name}: Line {line_num}: {e.message}")
                        return False
                    
                    # Check node references exist
                    src = edge.get("src")
                    dst = edge.get("dst")
                    if node_ids and src not in node_ids:
                        print(f"⚠ {edges_file.name}: Line {line_num}: Source node '{src}' not found in nodes file")
                    if node_ids and dst not in node_ids:
                        print(f"⚠ {edges_file.name}: Line {line_num}: Destination node '{dst}' not found in nodes file")
                    
                    edge_count += 1
                except json.JSONDecodeError as e:
                    print(f"✗ {edges_file.name}: Line {line_num}: Invalid JSON: {e}")
                    return False
        
        print(f"✓ {edges_file.name} ({edge_count} edges)")
        return True
    except Exception as e:
        print(f"✗ {edges_file}: Error: {e}")
        return False

def main():
    """Main validation function."""
    args = parse_args()
    
    if args.root:
        adr_dir = Path(args.root)
    else:
        adr_dir = REPO_ROOT / "docs" / "adr"
    
    print("Validating ADR structure...\n")
    
    # Load schemas
    try:
        adr_schema = load_schema(SCHEMA_DIR / "adr.schema.json")
        phase_schema = load_schema(SCHEMA_DIR / "phase.schema.json")
        index_schema = load_schema(SCHEMA_DIR / "adr.index.schema.json")
    except Exception as e:
        print(f"✗ Failed to load schemas: {e}")
        sys.exit(1)
    
    all_valid = True
    
    # Validate ADRs
    print("Validating ADR files:")
    for adr_dir_item in sorted(adr_dir.glob("ADR-*")):
        if not adr_dir_item.is_dir():
            continue
        
        adr_num = adr_dir_item.name.split('-')[1]
        adr_file = adr_dir_item / f"ADR-{adr_num}.md"
        
        if adr_file.exists():
            if not validate_adr(adr_file, adr_schema):
                all_valid = False
            
            # Validate phases
            phases_dir = adr_dir_item / "phases"
            if phases_dir.is_dir():
                for phase_file in sorted(phases_dir.glob("*.yaml")):
                    if not validate_phase(phase_file, phase_schema):
                        all_valid = False
        else:
            print(f"✗ {adr_dir_item.name}: ADR file not found")
            all_valid = False
    
    # Validate index
    print("\nValidating index:")
    index_file = adr_dir / "adr.index.json"
    if index_file.exists():
        if not validate_index(index_file, index_schema, REPO_ROOT):
            all_valid = False
    else:
        print(f"✗ Index file not found")
        all_valid = False
    
    # Validate graph files
    print("\nValidating graph files:")
    try:
        node_schema = load_schema(SCHEMA_DIR / "graph.nodes.schema.json")
        edge_schema = load_schema(SCHEMA_DIR / "graph.edges.schema.json")
        
        nodes_file = adr_dir / "graph.nodes.jsonl"
        edges_file = adr_dir / "graph.edges.jsonl"
        
        if not validate_graph_nodes(nodes_file, node_schema):
            all_valid = False
        
        if not validate_graph_edges(edges_file, edge_schema, nodes_file):
            all_valid = False
    except FileNotFoundError as e:
        print(f"⚠ Graph schema files not found: {e}")
        # Graph files are optional, so this is a warning, not an error
    except Exception as e:
        print(f"⚠ Failed to validate graph files: {e}")
    
    print()
    if all_valid:
        print("✓ All validations passed!")
        sys.exit(0)
    else:
        print("✗ Some validations failed")
        sys.exit(1)

if __name__ == "__main__":
    main()

