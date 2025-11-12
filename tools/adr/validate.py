#!/usr/bin/env python3
"""
Validate ADR structure and schemas.
"""

import json
import yaml
import sys
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

def validate_index(index_file: Path, index_schema: dict) -> bool:
    """Validate the ADR index."""
    try:
        with open(index_file, 'r', encoding='utf-8') as f:
            index_data = json.load(f)
        
        try:
            validate(instance=index_data, schema=index_schema)
            print(f"✓ {index_file.name}")
            return True
        except ValidationError as e:
            print(f"✗ {index_file.name}: {e.message}")
            return False
    except Exception as e:
        print(f"✗ {index_file}: Error: {e}")
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
        if not validate_index(index_file, index_schema):
            all_valid = False
    else:
        print(f"✗ Index file not found")
        all_valid = False
    
    print()
    if all_valid:
        print("✓ All validations passed!")
        sys.exit(0)
    else:
        print("✗ Some validations failed")
        sys.exit(1)

if __name__ == "__main__":
    main()

