#!/usr/bin/env python3
"""
Generate ADR index from all ADR directories.
"""

import json
import os
import yaml
import glob
from pathlib import Path
from datetime import datetime, timezone

REPO_ROOT = Path(__file__).parent.parent
ADR_DIR = REPO_ROOT / "docs" / "adr"
INDEX_FILE = ADR_DIR / "adr.index.json"

def generate_index():
    """Generate the ADR index."""
    print("Generating ADR index...")
    
    index = {
        "$schema": "./adr.index.schema.json",
        "version": "0.3",
        "generated_at": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "adrs": []
    }
    
    # Process each ADR directory
    for adr_dir in sorted(ADR_DIR.glob("ADR-*")):
        if not adr_dir.is_dir():
            continue
        
        # Find ADR markdown file
        adr_num = adr_dir.name.split('-')[1]
        adr_file = adr_dir / f"ADR-{adr_num}.md"
        
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
        phases_dir = adr_dir / "phases"
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
    
    # Write index
    with open(INDEX_FILE, 'w', encoding='utf-8') as f:
        json.dump(index, f, indent=2)
    
    print(f"✓ Generated ADR index: {INDEX_FILE}")
    print(f"  Found {len(index['adrs'])} ADRs")

if __name__ == "__main__":
    generate_index()

