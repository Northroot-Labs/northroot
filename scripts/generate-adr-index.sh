#!/bin/bash
# Generate ADR index from all ADR directories
# Usage: ./scripts/generate-adr-index.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
ADR_DIR="$REPO_ROOT/docs/adr"
INDEX_FILE="$ADR_DIR/adr.index.json"
TEMP_INDEX=$(mktemp)

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${GREEN}Generating ADR index...${NC}"

# Start JSON structure
cat > "$TEMP_INDEX" << 'EOF'
{
  "$schema": "./adr.index.schema.json",
  "version": "0.3",
  "generated_at": "",
  "adrs": []
}
EOF

# Get current timestamp in ISO 8601
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

# Use jq if available, otherwise use Python
if command -v jq &> /dev/null; then
    # Update timestamp
    jq --arg ts "$TIMESTAMP" '.generated_at = $ts' "$TEMP_INDEX" > "${TEMP_INDEX}.tmp" && mv "${TEMP_INDEX}.tmp" "$TEMP_INDEX"
    
    # Process each ADR directory
    for adr_dir in "$ADR_DIR"/ADR-*; do
        if [ ! -d "$adr_dir" ]; then
            continue
        fi
        
        adr_file="$adr_dir/ADR-$(basename "$adr_dir" | cut -d'-' -f2- | sed 's/-.*$//').md"
        
        if [ ! -f "$adr_file" ]; then
            echo -e "${YELLOW}Warning: ADR file not found: $adr_file${NC}"
            continue
        fi
        
        # Extract frontmatter (between --- markers)
        if ! grep -q "^---$" "$adr_file"; then
            echo -e "${YELLOW}Warning: No frontmatter found in $adr_file${NC}"
            continue
        fi
        
        # Extract YAML frontmatter
        frontmatter=$(awk '/^---$/{flag=!flag; next} flag' "$adr_file")
        
        # Parse with yq or Python
        if command -v yq &> /dev/null; then
            adr_json=$(echo "$frontmatter" | yq eval -o=json -)
        else
            # Fallback to Python
            adr_json=$(python3 << PYEOF
import yaml
import json
import sys
try:
    data = yaml.safe_load("""$frontmatter""")
    print(json.dumps(data))
except Exception as e:
    print('{}', file=sys.stderr)
    sys.exit(1)
PYEOF
)
        fi
        
        # Extract phases
        phases_dir="$adr_dir/phases"
        phases_json="[]"
        if [ -d "$phases_dir" ]; then
            phases_json=$(python3 << PYEOF
import json
import os
import glob
phases = []
for phase_file in sorted(glob.glob("$phases_dir/ADR-*-P*.yaml")):
    try:
        with open(phase_file, 'r') as f:
            import yaml
            phase_data = yaml.safe_load(f)
            if phase_data:
                phases.append({
                    "phase_id": phase_data.get("phase_id", ""),
                    "sequence": phase_data.get("sequence", 0),
                    "status": phase_data.get("status", "unknown"),
                    "timestamp": phase_data.get("decided_at") or phase_data.get("proposed_at") or ""
                })
    except Exception as e:
        pass
print(json.dumps(phases))
PYEOF
)
        fi
        
        # Add to index
        adr_id=$(echo "$adr_json" | python3 -c "import sys, json; print(json.load(sys.stdin).get('adr_id', ''))" 2>/dev/null || echo "")
        if [ -n "$adr_id" ]; then
            jq --argjson adr "$adr_json" --argjson phases "$phases_json" \
               '.adrs += [{
                 adr_id: $adr.adr_id,
                 slug: $adr.slug,
                 title: $adr.title,
                 status: $adr.status,
                 tags: ($adr.tags // []),
                 domain: ($adr.domain // ""),
                 created_at: $adr.created_at,
                 phases: $phases
               }]' "$TEMP_INDEX" > "${TEMP_INDEX}.tmp" && mv "${TEMP_INDEX}.tmp" "$TEMP_INDEX"
        fi
    done
else
    # Python-only implementation
    python3 << PYEOF
import json
import os
import glob
import yaml
from datetime import datetime

index = {
    "\$schema": "./adr.index.schema.json",
    "version": "0.3",
    "generated_at": "$TIMESTAMP",
    "adrs": []
}

adr_base = "$ADR_DIR"
for adr_dir in sorted(glob.glob(os.path.join(adr_base, "ADR-*"))):
    if not os.path.isdir(adr_dir):
        continue
    
    # Find ADR markdown file
    adr_num = os.path.basename(adr_dir).split('-')[1]
    adr_file = os.path.join(adr_dir, f"ADR-{adr_num}.md")
    
    if not os.path.exists(adr_file):
        print(f"Warning: ADR file not found: {adr_file}", file=sys.stderr)
        continue
    
    # Parse frontmatter
    with open(adr_file, 'r') as f:
        content = f.read()
        if not content.startswith('---'):
            print(f"Warning: No frontmatter in {adr_file}", file=sys.stderr)
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
            print(f"Warning: Failed to parse frontmatter in {adr_file}: {e}", file=sys.stderr)
            continue
    
    # Collect phases
    phases = []
    phases_dir = os.path.join(adr_dir, "phases")
    if os.path.isdir(phases_dir):
        for phase_file in sorted(glob.glob(os.path.join(phases_dir, "ADR-*-P*.yaml"))):
            try:
                with open(phase_file, 'r') as pf:
                    phase_data = yaml.safe_load(pf)
                    if phase_data:
                        phases.append({
                            "phase_id": phase_data.get("phase_id", ""),
                            "sequence": phase_data.get("sequence", 0),
                            "status": phase_data.get("status", "unknown"),
                            "timestamp": phase_data.get("decided_at") or phase_data.get("proposed_at") or ""
                        })
            except Exception as e:
                pass
    
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

# Write index
with open("$TEMP_INDEX", 'w') as f:
    json.dump(index, f, indent=2)
PYEOF
fi

# Sort ADRs by ADR ID
if command -v jq &> /dev/null; then
    jq '.adrs |= sort_by(.adr_id)' "$TEMP_INDEX" > "${TEMP_INDEX}.sorted" && mv "${TEMP_INDEX}.sorted" "$TEMP_INDEX"
else
    python3 << PYEOF
import json
with open("$TEMP_INDEX", 'r') as f:
    data = json.load(f)
data["adrs"].sort(key=lambda x: x["adr_id"])
with open("$TEMP_INDEX", 'w') as f:
    json.dump(data, f, indent=2)
PYEOF
fi

# Move to final location
mv "$TEMP_INDEX" "$INDEX_FILE"
echo -e "${GREEN}✓ Generated ADR index: $INDEX_FILE${NC}"

