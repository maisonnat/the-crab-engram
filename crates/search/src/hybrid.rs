/// Reciprocal Rank Fusion (RRF) for combining ranked lists.
///
/// Combines FTS5 results and vector similarity results into a single ranking.
/// k=60 is the standard constant from the RRF paper.
pub fn reciprocal_rank_fusion(
    fts_results: &[(i64, f64)],    // (observation_id, fts_rank)
    vector_results: &[(i64, f64)], // (observation_id, distance)
    k: usize,
    fts_weight: f64,
    vector_weight: f64,
) -> Vec<(i64, f64)> {
    use std::collections::HashMap;

    let mut scores: HashMap<i64, f64> = HashMap::new();

    // FTS contribution
    for (rank, (id, _score)) in fts_results.iter().enumerate() {
        let rrf_score = fts_weight / (k as f64 + rank as f64 + 1.0);
        *scores.entry(*id).or_insert(0.0) += rrf_score;
    }

    // Vector contribution (distance → rank-like score: lower distance = higher rank)
    let mut sorted_vector = vector_results.to_vec();
    sorted_vector.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    for (rank, (id, _distance)) in sorted_vector.iter().enumerate() {
        let rrf_score = vector_weight / (k as f64 + rank as f64 + 1.0);
        *scores.entry(*id).or_insert(0.0) += rrf_score;
    }

    // Sort by combined score descending
    let mut result: Vec<(i64, f64)> = scores.into_iter().collect();
    result.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    result
}

/// Compute final relevance score combining multiple signals.
pub fn compute_relevance_score(
    fts_rank_normalized: f64, // 0-1
    vector_similarity: f64,   // 0-1 (cosine similarity)
    recency_score: f64,       // 0-1
    frequency_score: f64,     // 0-1
) -> f64 {
    0.3 * fts_rank_normalized
        + 0.3 * vector_similarity
        + 0.2 * recency_score
        + 0.2 * frequency_score
}

/// Three-way RRF combining FTS5 + binary prefilter + full vector results.
///
/// Binary pre-filter shortlists candidates via Hamming distance,
/// then full cosine similarity re-ranks the shortlist.
/// This is much faster than scanning all full vectors.
pub fn reciprocal_rank_fusion_binary(
    fts_results: &[(i64, f64)],       // (observation_id, fts_rank)
    binary_results: &[(i64, f64)],    // (observation_id, hamming_similarity 0-1)
    vector_results: &[(i64, f64)],    // (observation_id, cosine_similarity)
    k: usize,
    fts_weight: f64,
    binary_weight: f64,
    vector_weight: f64,
) -> Vec<(i64, f64)> {
    use std::collections::HashMap;

    let mut scores: HashMap<i64, f64> = HashMap::new();

    // FTS contribution
    for (rank, (id, _score)) in fts_results.iter().enumerate() {
        let rrf_score = fts_weight / (k as f64 + rank as f64 + 1.0);
        *scores.entry(*id).or_insert(0.0) += rrf_score;
    }

    // Binary prefilter contribution (hamming similarity → rank)
    let mut sorted_binary = binary_results.to_vec();
    sorted_binary.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    for (rank, (id, _sim)) in sorted_binary.iter().enumerate() {
        let rrf_score = binary_weight / (k as f64 + rank as f64 + 1.0);
        *scores.entry(*id).or_insert(0.0) += rrf_score;
    }

    // Full vector contribution (cosine similarity → rank)
    let mut sorted_vector = vector_results.to_vec();
    sorted_vector.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    for (rank, (id, _sim)) in sorted_vector.iter().enumerate() {
        let rrf_score = vector_weight / (k as f64 + rank as f64 + 1.0);
        *scores.entry(*id).or_insert(0.0) += rrf_score;
    }

    // Sort by combined score descending
    let mut result: Vec<(i64, f64)> = scores.into_iter().collect();
    result.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rrf_combines_results() {
        let fts = vec![(1, 1.0), (2, 0.8), (3, 0.5)];
        let vector = vec![(2, 0.1), (4, 0.2), (1, 0.3)];

        let result = reciprocal_rank_fusion(&fts, &vector, 60, 0.4, 0.6);

        // Both 1 and 2 appear in both lists, should rank higher
        assert!(!result.is_empty());
        assert!(result[0].1 > 0.0);

        // IDs 1 and 2 should appear (present in both lists)
        let ids: Vec<i64> = result.iter().map(|(id, _)| *id).collect();
        assert!(ids.contains(&1));
        assert!(ids.contains(&2));
    }

    #[test]
    fn rrf_empty_lists() {
        let fts: Vec<(i64, f64)> = vec![];
        let vector = vec![(1, 0.1)];
        let result = reciprocal_rank_fusion(&fts, &vector, 60, 0.4, 0.6);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, 1);
    }

    #[test]
    fn relevance_score_weighted() {
        let score = compute_relevance_score(1.0, 0.8, 0.6, 0.4);
        let expected = 0.3 * 1.0 + 0.3 * 0.8 + 0.2 * 0.6 + 0.2 * 0.4;
        assert!((score - expected).abs() < 1e-10);
    }
}
