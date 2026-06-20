CREATE TABLE IF NOT EXISTS eth_to_bizinikiwi_blocks (
	ethereum_block_hash BLOB NOT NULL PRIMARY KEY,
	bizinikiwi_block_hash BLOB NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_bizinikiwi_block_hash ON eth_to_bizinikiwi_blocks (
	bizinikiwi_block_hash
);
