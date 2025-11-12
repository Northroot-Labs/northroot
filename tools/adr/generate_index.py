#!/usr/bin/env python3
"""
Generate ADR index from all ADR directories.
"""

import json
import os
import sys
import yaml
import glob
from pathlib import Path
from datetime import datetime, timezone

import argparse

def parse_args():
    """Parse command line arguments."""
    parser = argparse.ArgumentParser(description="Generate ADR index")
    parser.add_argument(
        "--root",
        type=str,
        default=None,
        help="Root directory for ADRs (default: auto-detect from script location)"
    )
    parser.add_argument(
        "--write",
        action="store_true",
        help="Write index file (default: just validate)"
    )
    return parser.parse_args()

# Auto-detect repo root and ADR directory
SCRIPT_DIR = Path(__file__).parent
REPO_ROOT = SCRIPT_DIR.parent.parent

def generate_index(adr_dir: Path, write: bool = True):
    """Generate the ADR index."""
    index_file = adr_dir / "adr.index.json"
    
    if write:
        print("Generating ADR index...")
    else:
        print("Validating ADR index...")
    
    index = {
        "$schema": "./adr.index.schema.json",
        "version": "0.3",
        "generated_at": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "adrs": []
    }
    
    # Process each ADR directory
    for adr_dir_item in sorted(adr_dir.glob("ADR-*")):
        if not adr_dir_item.is_dir():
            continue
        
        # Find ADR markdown file
        adr_num = adr_dir_item.name.split('-')[1]
        adr_file = adr_dir_item / f"ADR-{adr_num}.md"
        
        if not adr_file.exists():
            print(f"Warning: ADR file not found: {adr_file}")
            continue
        
        # Parse frontmatter
        with open(adr_file, 'r', encoding='utf-8') as f:
            content = f.read()
            if not content.startswith('---'):
                print(f"Warning: No frontmatter in {adr_file}")
                continue
            
            # Extract YAML frontmatter
            parts = content.split('---', 2)
            if len(parts) < 3:
                continue
            
            try:
                frontmatter = yaml.safe_load(parts[1])
                if not frontmatter:
                    continue
            except Exception as e:
                print(f"Warning: Failed to parse frontmatter in {adr_file}: {e}")
                continue
        
        # Collect phases
        phases = []
        phases_dir = adr_dir_item / "phases"
        if phases_dir.is_dir():
            for phase_file in sorted(phases_dir.glob("ADR-*-P*.yaml")):
                try:
                    with open(phase_file, 'r', encoding='utf-8') as pf:
                        phase_data = yaml.safe_load(pf)
                        if phase_data:
                            phases.append({
                                "phase_id": phase_data.get("phase_id", ""),
                                "sequence": phase_data.get("sequence", 0),
                                "status": phase_data.get("status", "unknown"),
                                "timestamp": phase_data.get("decided_at") or phase_data.get("proposed_at") or ""
                            })
                except Exception as e:
                    print(f"Warning: Failed to parse phase file {phase_file}: {e}")
        
        # Add to index
        index["adrs"].append({
            "adr_id": frontmatter.get("adr_id", ""),
            "slug": frontmatter.get("slug", ""),
            "title": frontmatter.get("title", ""),
            "status": frontmatter.get("status", "unknown"),
            "tags": frontmatter.get("tags", []),
            "domain": frontmatter.get("domain", ""),
            "created_at": frontmatter.get("created_at", ""),
            "phases": phases
        })
    
    # Sort by ADR ID
    index["adrs"].sort(key=lambda x: x["adr_id"])
    
    if write:
        # Write index
        with open(index_file, 'w', encoding='utf-8') as f:
            json.dump(index, f, indent=2)
        print(f"✓ Generated ADR index: {index_file}")
    else:
        # Validate existing index
        if not index_file.exists():
            print(f"✗ Index file not found: {index_file}")
            return False
        
        with open(index_file, 'r', encoding='utf-8') as f:
            existing = json.load(f)
        
        # Compare (ignore generated_at)
        existing_copy = existing.copy()
        index_copy = index.copy()
        existing_copy.pop("generated_at", None)
        index_copy.pop("generated_at", None)
        
        if existing_copy != index_copy:
            print(f"✗ Index is out of date")
            return False
        print(f"✓ Index is up to date")
    
    print(f"  Found {len(index['adrs'])} ADRs")
    return True

if __name__ == "__main__":
    args = parse_args()
    if args.root:
        adr_dir = Path(args.root)
    else:
        adr_dir = REPO_ROOT / "docs" / "adr"
    
    success = generate_index(adr_dir, write=args.write)
    sys.exit(0 if success else 1)

