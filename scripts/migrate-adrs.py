#!/usr/bin/env python3
"""
Migrate existing ADRs to new format with directory structure, YAML frontmatter, and phase files.
"""

import os
import re
import json
import yaml
import shutil
from pathlib import Path
from datetime import datetime, timezone
from typing import Dict, List, Optional, Tuple

REPO_ROOT = Path(__file__).parent.parent
OLD_ADR_DIR = REPO_ROOT / "ADRs"
NEW_ADR_DIR = REPO_ROOT / "docs" / "adr"
SCHEMA_DIR = REPO_ROOT / "schemas" / "adr"

# ADR metadata extraction patterns
ADR_PATTERNS = {
    "title": re.compile(r"^#\s*ADR-(\d+):\s*(.+)$", re.MULTILINE),
    "date": re.compile(r"\*\*Date:\*\*\s*(.+?)(?:\n|$)", re.IGNORECASE),
    "status": re.compile(r"\*\*Status:\*\*\s*(.+?)(?:\n|$)", re.IGNORECASE),
    "context": re.compile(r"\*\*Context:\*\*\s*(.+?)(?:\n|$)", re.IGNORECASE),
}

# Slug generation from title
def generate_slug(title: str) -> str:
    """Generate URL-friendly slug from title."""
    slug = title.lower()
    slug = re.sub(r'[^a-z0-9\s-]', '', slug)
    slug = re.sub(r'\s+', '-', slug)
    slug = slug.strip('-')
    return slug[:50]  # Limit length

# Extract ADR number from filename
def extract_adr_number(filename: str) -> str:
    """Extract ADR number from filename (e.g., ADR-001-receipts-vs-engine.md -> 0001)."""
    match = re.search(r'ADR-(\d+)', filename)
    if match:
        return match.group(1).zfill(4)
    return "0000"

# Parse existing ADR content
def parse_adr_file(filepath: Path) -> Dict:
    """Parse existing ADR markdown file and extract metadata."""
    content = filepath.read_text(encoding='utf-8')
    
    # Extract ADR number
    adr_num = extract_adr_number(filepath.name)
    adr_id = f"ADR-{adr_num}"
    
    # Extract title
    title_match = ADR_PATTERNS["title"].search(content)
    if title_match:
        title = title_match.group(2).strip()
    else:
        title = filepath.stem.replace(f"ADR-{adr_num}", "").strip("-").replace("-", " ").title()
    
    # Extract date
    date_match = ADR_PATTERNS["date"].search(content)
    if date_match:
        date_str = date_match.group(1).strip()
        # Try to parse various date formats
        try:
            if "(" in date_str:
                date_str = date_str.split("(")[0].strip()
            dt = datetime.strptime(date_str, "%Y-%m-%d")
            created_at = dt.strftime("%Y-%m-%dT%H:%M:%SZ")
        except:
            created_at = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    else:
        # Use file modification time
        mtime = os.path.getmtime(filepath)
        created_at = datetime.fromtimestamp(mtime).strftime("%Y-%m-%dT%H:%M:%SZ")
    
    # Extract status
    status_match = ADR_PATTERNS["status"].search(content)
    if status_match:
        status_str = status_match.group(1).strip()
        # Normalize status
        status_lower = status_str.lower()
        if "accepted" in status_lower:
            status = "accepted"
        elif "proposed" in status_lower:
            status = "proposed"
        elif "rejected" in status_lower:
            status = "rejected"
        elif "deprecated" in status_lower or "superseded" in status_lower:
            status = "deprecated"
        else:
            status = "proposed"
    else:
        status = "proposed"
    
    # Extract context/problem
    context_match = ADR_PATTERNS["context"].search(content)
    problem = context_match.group(1).strip() if context_match else ""
    
    # Generate slug
    slug = generate_slug(title)
    
    # Extract domain from content (heuristic)
    domain = "platform"  # default
    if "engine" in content.lower() or "receipt" in content.lower():
        domain = "engine"
    elif "storage" in content.lower():
        domain = "storage"
    elif "canonical" in content.lower() or "cbor" in content.lower():
        domain = "core"
    
    # Extract tags (heuristic)
    tags = []
    content_lower = content.lower()
    if "cbor" in content_lower or "canonical" in content_lower:
        tags.append("canonicalization")
    if "delta" in content_lower or "incremental" in content_lower:
        tags.append("delta-compute")
    if "storage" in content_lower:
        tags.append("storage")
    if "receipt" in content_lower:
        tags.append("receipts")
    if "engine" in content_lower:
        tags.append("engine")
    if "proof" in content_lower:
        tags.append("proofs")
    
    return {
        "adr_id": adr_id,
        "slug": slug,
        "title": title,
        "created_at": created_at,
        "status": status,
        "domain": domain,
        "tags": tags,
        "problem": problem,
        "content": content
    }

# Create new ADR structure
def create_adr_structure(adr_data: Dict, content: str) -> None:
    """Create new ADR directory structure with markdown and frontmatter."""
    adr_id = adr_data["adr_id"]
    slug = adr_data["slug"]
    
    # Create directory
    adr_dir = NEW_ADR_DIR / f"{adr_id}-{slug}"
    adr_dir.mkdir(parents=True, exist_ok=True)
    (adr_dir / "phases").mkdir(exist_ok=True)
    (adr_dir / "attachments").mkdir(exist_ok=True)
    
    # Create frontmatter
    frontmatter = {
        "$schema": "../../schemas/adr/adr.schema.json",
        "adr_id": adr_id,
        "slug": slug,
        "title": adr_data["title"],
        "created_at": adr_data["created_at"],
        "status": adr_data["status"],
        "domain": adr_data["domain"],
        "tags": adr_data["tags"],
        "related": {
            "supersedes": [],
            "superseded_by": [],
            "depends_on": [],
            "related_to": []
        }
    }
    
    if adr_data.get("problem"):
        frontmatter["decision_context"] = {
            "problem": adr_data["problem"]
        }
    
    # Write ADR markdown with frontmatter
    # ADR ID format: ADR-0001, so split and take the number part
    adr_num = adr_id.split('-')[1]  # e.g., "0001"
    adr_file = adr_dir / f"ADR-{adr_num}.md"
    with open(adr_file, 'w', encoding='utf-8') as f:
        f.write("---\n")
        f.write(yaml.dump(frontmatter, default_flow_style=False, sort_keys=False))
        f.write("---\n\n")
        # Write original content (skip old frontmatter-like sections)
        f.write(content)
    
    print(f"✓ Created {adr_dir}")

# Create phase file for ADR-009
def create_adr009_phases(adr_dir: Path) -> None:
    """Create phase files for ADR-009 based on existing phase documentation."""
    phases_dir = adr_dir / "phases"
    
    # Phase data from ADR-009
    phases = [
        {
            "sequence": 1,
            "title": "Engine-internal DataShape enum + hash helper",
            "status": "implemented",
            "summary": "Added shapes.rs module with DataShape enum and compute_data_shape_hash() function."
        },
        {
            "sequence": 2,
            "title": "Extend ExecutionPayload to differentiate byte-level commitments",
            "status": "implemented",
            "summary": "Updated ExecutionPayload with output_digest, manifest_root, and locator refs."
        },
        {
            "sequence": 3,
            "title": "Refactor Merkle Row-Map and ByteStream manifest builders",
            "status": "implemented",
            "summary": "Moved MerkleRowMap to rowmap.rs with RFC-6962 domain separation. Created cas.rs for ByteStream manifests."
        },
        {
            "sequence": 4,
            "title": "Privacy-Preserving Resolver API",
            "status": "implemented",
            "summary": "Created resolver.rs module with ArtifactResolver and ManagedCache traits."
        },
        {
            "sequence": 5,
            "title": "Summarized manifests for fast overlap",
            "status": "proposed",
            "summary": "Implement ManifestSummary structure with MinHash sketches, HLL cardinality, and Bloom filters."
        },
        {
            "sequence": 6,
            "title": "Storage extensions",
            "status": "implemented",
            "summary": "Extended ReceiptStore trait with encrypted locators, output digests, and manifest summaries."
        },
        {
            "sequence": 7,
            "title": "Reuse reconciliation flow",
            "status": "proposed",
            "summary": "Implement ReuseReconciliation + check_reuse() helpers with manifest summaries."
        },
        {
            "sequence": 8,
            "title": "Helper functions for shape hash computation",
            "status": "proposed",
            "summary": "Add data shape and method shape hash helpers."
        }
    ]
    
    for phase in phases:
        phase_id = f"ADR-0009-P{phase['sequence']:02d}"
        phase_file = phases_dir / f"{phase_id}.yaml"
        
        phase_data = {
            "$schema": "../../../schemas/adr/phase.schema.json",
            "phase_id": phase_id,
            "adr_id": "ADR-0009",
            "sequence": phase["sequence"],
            "title": phase["title"],
            "status": phase["status"],
            "summary": phase["summary"],
            "proposed_at": "2025-11-11T00:00:00Z" if phase["status"] != "proposed" else datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
            "decided_at": "2025-11-11T00:00:00Z" if phase["status"] == "implemented" else None
        }
        
        # Remove None values
        phase_data = {k: v for k, v in phase_data.items() if v is not None}
        
        with open(phase_file, 'w', encoding='utf-8') as f:
            yaml.dump(phase_data, f, default_flow_style=False, sort_keys=False)
        
        print(f"  ✓ Created phase {phase_id}")

def main():
    """Main migration function."""
    print("Migrating ADRs to new format...")
    print(f"Source: {OLD_ADR_DIR}")
    print(f"Destination: {NEW_ADR_DIR}\n")
    
    # Ensure new directory exists
    NEW_ADR_DIR.mkdir(parents=True, exist_ok=True)
    
    # Process each ADR file
    adr_files = sorted(OLD_ADR_DIR.glob("ADR-*.md"))
    
    for adr_file in adr_files:
        print(f"Processing {adr_file.name}...")
        try:
            adr_data = parse_adr_file(adr_file)
            create_adr_structure(adr_data, adr_data["content"])
            
            # Special handling for ADR-009 (has phases)
            if adr_data["adr_id"] == "ADR-0009":
                adr_dir = NEW_ADR_DIR / f"{adr_data['adr_id']}-{adr_data['slug']}"
                create_adr009_phases(adr_dir)
        except Exception as e:
            print(f"  ✗ Error processing {adr_file.name}: {e}")
            import traceback
            traceback.print_exc()
    
    print("\n✓ Migration complete!")
    print(f"\nNext steps:")
    print(f"1. Review migrated ADRs in {NEW_ADR_DIR}")
    print(f"2. Run: ./scripts/generate-adr-index.sh")
    print(f"3. Update any cross-references in code/docs")

if __name__ == "__main__":
    main()

