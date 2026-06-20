
function display {
  echo "comparing $1 -> $2"
  subweight compare files \
    --method asymptotic \
    --new $1 \
    --old $2 \
    --unit proof --verbose --threshold 0

  subweight compare files \
    --method asymptotic \
    --new $1 \
    --old $2 \
    --unit time --verbose --threshold 0
}

## Pezkuwi
display "./pallet_election_provider_multi_block_hez_size.rs" "./pallet_election_provider_multi_block_ksm_size.rs"
display "./pallet_election_provider_multi_block_signed_hez_size.rs" "./pallet_election_provider_multi_block_signed_ksm_size.rs"
display "./pallet_election_provider_multi_block_unsigned_hez_size.rs" "./pallet_election_provider_multi_block_unsigned_ksm_size.rs"
display "./pallet_election_provider_multi_block_verifier_hez_size.rs" "./pallet_election_provider_multi_block_verifier_ksm_size.rs"
