#!/usr/bin/env bash

export PRODUCT=pezkuwi
export VERSION=${VERSION:-stable2409}
export ENGINE=${ENGINE:-podman}
export REF1=${REF1:-'HEAD'}
export REF2=${REF2}
export RUSTC_STABLE=${RUSTC_STABLE:-'1.0'}
export NO_RUNTIMES=${NO_RUNTIMES:-'false'}
export CRATES_ONLY=${CRATES_ONLY:-'false'}

PROJECT_ROOT=`git rev-parse --show-toplevel`
echo $PROJECT_ROOT

TMP=${TMP:-$(mktemp -d)}
TEMPLATE_AUDIENCE="${PROJECT_ROOT}/scripts/release/templates/audience.md.tera"
TEMPLATE_CHANGELOG="${PROJECT_ROOT}/scripts/release/templates/changelog.md.tera"

DATA_JSON="${TMP}/data.json"
CONTEXT_JSON="${TMP}/context.json"
echo -e "TEMPLATE_AUDIENCE: \t$TEMPLATE_AUDIENCE"
echo -e "DATA_JSON: \t\t$DATA_JSON"
echo -e "CONTEXT_JSON: \t\t$CONTEXT_JSON"

# Create output folder
OUTPUT="${TMP}/changelogs/$PRODUCT/$VERSION"
echo -e "OUTPUT: \t\t$OUTPUT"
mkdir -p $OUTPUT

$ENGINE run --rm -v ${PROJECT_ROOT}:/repo pezkuwichain/prdoc load -d "prdoc/$VERSION" --json > $DATA_JSON

cat $DATA_JSON | jq ' { "prdoc" : .}' > $CONTEXT_JSON

# Fetch the list of valid audiences and their descriptions
SCHEMA_URL=https://raw.githubusercontent.com/pezkuwichain/kurdistan-sdk/master/prdoc/schema_user.json
SCHEMA=$(curl -s $SCHEMA_URL | sed 's|^//.*||')
aud_desc_array=()
while IFS= read -r line; do
    audience=$(jq -r '.const' <<< "$line" )
    description=$(jq -r '.description' <<< "$line")
    if [ -n "$audience" ] && [ -n "$description" ]; then
        aud_desc_array+=("($audience; $description)")
    fi
done < <(jq -c '."$defs".audience_id.oneOf[]' <<< "$SCHEMA")

# Generate a release notes doc per audience
for tuple in "${aud_desc_array[@]}"; do
    audience=$(echo "$tuple" | cut -d ';' -f 1 | sed 's/(//')
    audience_id="$(tr [A-Z] [a-z] <<< "$audience")"
    audience_id="$(tr ' ' '_' <<< "$audience_id")"

    description=$(echo "$tuple" | cut -d ';' -f 2 | sed 's/)//')

    echo "Processing audience: $audience ($audience_id)"
    export TARGET_AUDIENCE="$audience"
    export AUDIENCE_DESC="**ℹ️ These changes are relevant to:** $description"

    tera -t "${TEMPLATE_AUDIENCE}" --env --env-key env "${CONTEXT_JSON}" > "$OUTPUT/relnote_${audience_id}.md"
    cat "$OUTPUT/relnote_${audience_id}.md" >> "$PROJECT_ROOT/scripts/release/templates/changelog.md"
done


# Generate a changelog containing list of the commits
echo "Generating changelog..."
tera -t "${TEMPLATE_CHANGELOG}" --env --env-key env "${CONTEXT_JSON}" > "$OUTPUT/relnote_commits.md"
echo "Changelog ready in $OUTPUT/relnote_commits.md"

# Show the files
tree -s -h -c $OUTPUT/

if [[ "$NO_RUNTIMES" == "false" && "$CRATES_ONLY" == "false" ]]; then
  ASSET_HUB_ZAGROS_DIGEST=${ASSET_HUB_ZAGROS_DIGEST:-"$PROJECT_ROOT/scripts/release/digests/asset-hub-zagros-srtool-digest.json"}
  BRIDGE_HUB_ZAGROS_DIGEST=${BRIDGE_HUB_ZAGROS_DIGEST:-"$PROJECT_ROOT/scripts/release/digests/bridge-hub-zagros-srtool-digest.json"}
  COLLECTIVES_ZAGROS_DIGEST=${COLLECTIVES_ZAGROS_DIGEST:-"$PROJECT_ROOT/scripts/release/digests/collectives-zagros-srtool-digest.json"}
  CORETIME_ZAGROS_DIGEST=${CORETIME_ZAGROS_DIGEST:-"$PROJECT_ROOT/scripts/release/digests/coretime-zagros-srtool-digest.json"}
  GLUTTON_ZAGROS_DIGEST=${GLUTTON_ZAGROS_DIGEST:-"$PROJECT_ROOT/scripts/release/digests/glutton-zagros-srtool-digest.json"}
  PEOPLE_ZAGROS_DIGEST=${PEOPLE_ZAGROS_DIGEST:-"$PROJECT_ROOT/scripts/release/digests/people-zagros-srtool-digest.json"}
  ZAGROS_DIGEST=${ZAGROS_DIGEST:-"$PROJECT_ROOT/scripts/release/digests/zagros-srtool-digest.json"}

  jq \
        --slurpfile srtool_asset_hub_zagros $ASSET_HUB_ZAGROS_DIGEST \
        --slurpfile srtool_bridge_hub_zagros $BRIDGE_HUB_ZAGROS_DIGEST \
        --slurpfile srtool_collectives_zagros $COLLECTIVES_ZAGROS_DIGEST \
        --slurpfile srtool_coretime_zagros $CORETIME_ZAGROS_DIGEST \
        --slurpfile srtool_glutton_zagros $GLUTTON_ZAGROS_DIGEST \
        --slurpfile srtool_people_zagros $PEOPLE_ZAGROS_DIGEST \
        --slurpfile srtool_zagros $ZAGROS_DIGEST \
        -n '{
            srtool: [
              { order: 10, name: "Zagros", data: $srtool_zagros[0] },
              { order: 11, name: "Zagros AssetHub", data: $srtool_asset_hub_zagros[0] },
              { order: 12, name: "Zagros BridgeHub", data: $srtool_bridge_hub_zagros[0] },
              { order: 13, name: "Zagros Collectives", data: $srtool_collectives_zagros[0] },
              { order: 14, name: "Zagros Coretime", data: $srtool_coretime_zagros[0] },
              { order: 15, name: "Zagros Glutton", data: $srtool_glutton_zagros[0] },
              { order: 16, name: "Zagros People", data: $srtool_people_zagros[0] }
        ] }' > "$PROJECT_ROOT/scripts/release/context.json"
else
  echo '{}' > "$PROJECT_ROOT/scripts/release/context.json"
fi

RELEASE_DIR="$PROJECT_ROOT/scripts/release/"
pushd $RELEASE_DIR >/dev/null
tera --env --env-key env --include-path templates --template templates/template.md.tera context.json > RELEASE_DRAFT.md
popd >/dev/null
