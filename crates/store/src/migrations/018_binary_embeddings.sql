-- Binary embedding support: add binary_hash column for 1-bit quantized embeddings.
-- Stores the binary-packed version of the embedding vector (48 bytes for 384 dims).
-- Enables fast pre-filtering via Hamming distance before full cosine similarity.

ALTER TABLE embeddings ADD COLUMN binary_hash BLOB;
