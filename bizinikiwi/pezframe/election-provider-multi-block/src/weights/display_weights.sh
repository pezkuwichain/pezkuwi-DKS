
function display {
  echo "displaying $1"
  subweight compare files \
    --method asymptotic \
    --new $1 \
    --old $1 \
    --unit proof \
    --verbose \
    --threshold 0

  subweight compare files \
    --method asymptotic \
    --new $1 \
    --old $1 \
    --unit time \
    --verbose \
    --threshold 0
}

## Pezkuwi

display "pallet_election_provider_multi_block_hez_size.rs"
display "pallet_election_provider_multi_block_signed_hez_size.rs"
display "pallet_election_provider_multi_block_unsigned_hez_size.rs"
display "pallet_election_provider_multi_block_verifier_hez_size.rs"

## Kusama
display "pallet_election_provider_multi_block_ksm_size.rs"
display "pallet_election_provider_multi_block_signed_ksm_size.rs"
display "pallet_election_provider_multi_block_unsigned_ksm_size.rs"
display "pallet_election_provider_multi_block_verifier_ksm_size.rs"
