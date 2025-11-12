#!/usr/bin/env python3
"""
Generate ADR index from all ADR directories.
"""

import json
import os
import sys
import yaml
import glob
import subprocess
import re
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

def get_version():
    """Get current version from Cargo.toml."""
    cargo_toml = REPO_ROOT / "Cargo.toml"
    if cargo_toml.exists():
        with open(cargo_toml, 'r') as f:
            content = f.read()
            match = re.search(r'version\s*=\s*"([^"]+)"', content)
            if match:
                return match.group(1)
    return "0.1.0"  # Default fallback

def get_git_commit(file_path: Path):
    """Get git commit SHA for a file (most recent commit that touched it)."""
    try:
        result = subprocess.run(
            ["git", "log", "-1", "--format=%H", "--", str(file_path)],
            cwd=REPO_ROOT,
            capture_output=True,
            text=True,
            check=False
        )
        if result.returncode == 0 and result.stdout.strip():
            return result.stdout.strip()
    except Exception:
        pass
    return None

def get_current_commit():
    """Get current git commit SHA."""
    try:
        result = subprocess.run(
            ["git", "rev-parse", "HEAD"],
            cwd=REPO_ROOT,
            capture_output=True,
            text=True,
            check=False
        )
        if result.returncode == 0 and result.stdout.strip():
            return result.stdout.strip()
    except Exception:
        pass
    return None

def generate_index(adr_dir: Path, write: bool = True):
    """Generate the ADR index."""
    index_file = adr_dir / "adr.index.json"
    
    if write:
        print("Generating ADR index...")
    else:
        print("Validating ADR index...")
    
    index = {
        "$schema": "./adr.index.schema.json",
        "version": "0.4",
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
        
        # Collect phases and track implementation metadata
        phases = []
        phase_ids = []
        first_implemented_at = None
        all_phases_implemented = True
        phases_dir = adr_dir_item / "phases"
        current_version = get_version()
        current_commit = get_current_commit()
        
        if phases_dir.is_dir():
            for phase_file in sorted(phases_dir.glob("ADR-*-P*.yaml")):
                try:
                    with open(phase_file, 'r', encoding='utf-8') as pf:
                        phase_data = yaml.safe_load(pf) or {}
                    
                    phase_status = phase_data.get("status", "unknown")
                    phase_id = phase_data.get("phase_id", "")
                    if phase_id:
                        phase_ids.append(phase_id)
                    
                    needs_write = False
                    
                    # Auto-set implemented_at if status is "implemented" and not already set
                    if phase_status == "implemented" and not phase_data.get("implemented_at"):
                        phase_data["implemented_at"] = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
                        needs_write = True
                    
                    # Auto-set implemented_in if status is "implemented" and not already set
                    if phase_status == "implemented" and not phase_data.get("implemented_in"):
                        phase_data["implemented_in"] = current_version
                        needs_write = True
                    
                    # Auto-set implementation_commit if status is "implemented" and not already set
                    if phase_status == "implemented" and not phase_data.get("implementation_commit"):
                        commit_sha = get_git_commit(phase_file) or current_commit
                        if commit_sha:
                            phase_data["implementation_commit"] = commit_sha
                            needs_write = True
                    
                    # Write back to file if we made changes
                    if needs_write and write:
                        # Read original file to preserve format
                        with open(phase_file, 'r', encoding='utf-8') as orig:
                            original_lines = orig.readlines()
                        
                        # Write back preserving order, updating only changed fields
                        with open(phase_file, 'w', encoding='utf-8') as pf:
                            for line in original_lines:
                                # Skip lines for fields we're updating
                                if line.startswith("implemented_at:") or line.startswith("implemented_in:") or line.startswith("implementation_commit:"):
                                    continue
                                pf.write(line)
                            
                            # Append new fields at end (before any trailing newlines)
                            if "implemented_at" in phase_data:
                                pf.write(f"implemented_at: '{phase_data['implemented_at']}'\n")
                            if "implemented_in" in phase_data:
                                pf.write(f"implemented_in: {phase_data['implemented_in']}\n")
                            if "implementation_commit" in phase_data:
                                pf.write(f"implementation_commit: {phase_data['implementation_commit']}\n")
                    
                    # Track first implemented phase for realized_at
                    if phase_status == "implemented":
                        implemented_at = phase_data.get("implemented_at")
                        if implemented_at and (not first_implemented_at or implemented_at < first_implemented_at):
                            first_implemented_at = implemented_at
                    else:
                        all_phases_implemented = False
                    
                    # Build phase entry for index
                    phase_entry = {
                                "phase_id": phase_id,
                                "sequence": phase_data.get("sequence", 0),
                        "status": phase_status,
                                "timestamp": phase_data.get("decided_at") or phase_data.get("proposed_at") or ""
                    }
                    
                    # Add implementation metadata if implemented
                    if phase_status == "implemented":
                        if phase_data.get("implemented_at"):
                            phase_entry["implemented_at"] = phase_data["implemented_at"]
                        if phase_data.get("implemented_in"):
                            phase_entry["implemented_in"] = phase_data["implemented_in"]
                        if phase_data.get("implementation_commit"):
                            phase_entry["implementation_commit"] = phase_data["implementation_commit"]
                    
                    # Add new metadata fields
                    # Normalize array fields to always be strings (handle YAML parsing issues)
                    def normalize_string_array(arr):
                        """Convert array that may contain dicts to array of strings."""
                        if not arr:
                            return []
                        result = []
                        for item in arr:
                            if isinstance(item, str):
                                result.append(item)
                            elif isinstance(item, dict):
                                # Handle YAML parsing issue where key-value pairs become dicts
                                # Convert dict entries to strings
                                for key, value in item.items():
                                    if value:
                                        result.append(f"{key}: {value}")
                                    else:
                                        result.append(key)
                        return result
                    
                    if phase_data.get("decision_drivers"):
                        drivers = normalize_string_array(phase_data["decision_drivers"])
                        if drivers:
                            phase_entry["decision_drivers"] = drivers
                    
                    if phase_data.get("tasks"):
                        tasks = normalize_string_array(phase_data["tasks"])
                        if tasks:
                            phase_entry["tasks"] = tasks
                    
                    if phase_data.get("files"):
                        files = normalize_string_array(phase_data["files"])
                        if files:
                            phase_entry["files"] = files
                    
                    if phase_data.get("success_criteria"):
                        criteria = normalize_string_array(phase_data["success_criteria"])
                        if criteria:
                            phase_entry["success_criteria"] = criteria
                    
                    phases.append(phase_entry)
                except Exception as e:
                    print(f"Warning: Failed to parse phase file {phase_file}: {e}")
        
        # Build ADR entry
        adr_entry = {
            "adr_id": frontmatter.get("adr_id", ""),
            "slug": frontmatter.get("slug", ""),
            "title": frontmatter.get("title", ""),
            "status": frontmatter.get("status", "unknown"),
            "tags": frontmatter.get("tags", []),
            "domain": frontmatter.get("domain", ""),
            "created_at": frontmatter.get("created_at", ""),
            "phases": phases
        }
        
        # Add new metadata fields
        if frontmatter.get("accepted_at"):
            adr_entry["accepted_at"] = frontmatter.get("accepted_at")
        elif frontmatter.get("status") == "accepted":
            # If status is accepted but no accepted_at, use created_at as fallback
            adr_entry["accepted_at"] = frontmatter.get("created_at", "")
        
        # Add implemented_at if all phases are implemented
        if all_phases_implemented and phases:
            # Use the latest implemented_at from phases
            latest_implemented = None
            for phase in phases:
                if phase.get("implemented_at"):
                    if not latest_implemented or phase["implemented_at"] > latest_implemented:
                        latest_implemented = phase["implemented_at"]
            if latest_implemented:
                adr_entry["implemented_at"] = latest_implemented
        
        # Add phase_ids
        if phase_ids:
            adr_entry["phase_ids"] = phase_ids
        
        # Add depends_on from related.depends_on
        related = frontmatter.get("related", {})
        if related.get("depends_on"):
            adr_entry["depends_on"] = related["depends_on"]
        
        # Add code_refs (initially empty, populated by linker)
        adr_entry["code_refs"] = []
        
        # Add realized_at if any phase is implemented
        if first_implemented_at:
            adr_entry["realized_at"] = first_implemented_at
        
        index["adrs"].append(adr_entry)
    
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

