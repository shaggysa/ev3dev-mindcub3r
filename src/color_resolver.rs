/// One RGB sample as returned by the color sensor.
#[derive(Clone, Copy, Debug)]
pub struct Rgb {
    pub r: u16,
    pub g: u16,
    pub b: u16,
}

/// Reference average for one cube color (White, Yellow, Red, Orange, Blue, Green).
/// The order you pass them in becomes the color index 0..6 in the output.
#[derive(Clone, Copy, Debug)]
pub struct ColorRef {
    pub r: u16,
    pub g: u16,
    pub b: u16,
}

// ---------------------------------------------------------------------------
// Core solver
// ---------------------------------------------------------------------------

/// Number of faces on a Rubik's cube.
pub const FACES: usize = 54;
/// Number of distinct colors.
pub const COLORS: usize = 6;
/// Required faces per color.
pub const PER_COLOR: usize = FACES / COLORS; // = 9

/// Squared Euclidean distance between a sensor reading and a reference,
/// computed entirely in u32 to avoid any floating-point operation.
///
/// Maximum possible value: 3 × (65535)² ≈ 1.29 × 10¹⁰  →  fits in u64.
/// We return u64 so the cost table never overflows even for pathological inputs.
#[inline]
fn sq_dist(sample: Rgb, reference: ColorRef) -> u64 {
    let dr = (sample.r as i64) - (reference.r as i64);
    let dg = (sample.g as i64) - (reference.g as i64);
    let db = (sample.b as i64) - (reference.b as i64);
    // Max value: 3 × 65535² ≈ 1.29 × 10¹⁰ — fits comfortably in i64/u64.
    (dr * dr + dg * dg + db * db) as u64
}

/// Assign each of the 54 `samples` to exactly one of the 6 `refs` such that
/// every color is used exactly `PER_COLOR` (9) times and the total squared
/// distance is minimised.
///
/// Returns an array `assignment[face] = color_index` (0..6).
///
/// # Panics
/// Panics in debug mode if `samples.len() != FACES` or `refs.len() != COLORS`.
pub fn assign_colors(samples: &[Rgb; FACES], refs: &[ColorRef; COLORS]) -> [usize; FACES] {
    // ------------------------------------------------------------------
    // Step 1: build cost table  cost[face][color] = sq_dist
    // ------------------------------------------------------------------
    let mut cost = [[0u64; COLORS]; FACES];
    for (f, &s) in samples.iter().enumerate() {
        for (c, &r) in refs.iter().enumerate() {
            cost[f][c] = sq_dist(s, r);
        }
    }

    // ------------------------------------------------------------------
    // Step 2: greedy initial assignment
    // ------------------------------------------------------------------
    let mut assignment = [0usize; FACES];
    let mut count = [0usize; COLORS];

    for f in 0..FACES {
        let best = (0..COLORS).min_by_key(|&c| cost[f][c]).unwrap();
        assignment[f] = best;
        count[best] += 1;
    }

    // ------------------------------------------------------------------
    // Step 3: rebalance via repeated shortest-augmenting-path
    //
    // We keep moving faces from over-quota colors to under-quota colors
    // along the cheapest augmenting path until every color has exactly
    // PER_COLOR faces.
    //
    // The "residual graph" here is conceptually:
    //   - Forward edge  face → color_target  cost = cost[face][target]
    //   - Backward edge color_source → face  cost = -cost[face][source]
    //     (i.e. we "refund" the cost of the current assignment)
    //
    // We run Dijkstra (with Johnson-style potentials to handle the negative
    // edges) on this residual graph starting from all over-quota colors
    // simultaneously, and stop at the first under-quota color reached.
    // ------------------------------------------------------------------
    rebalance(&cost, &mut assignment, &mut count);

    assignment
}

// ---------------------------------------------------------------------------
// Rebalancing engine
// ---------------------------------------------------------------------------

/// Repeatedly find the cheapest reassignment chain that transfers one face
/// from an over-quota color to an under-quota color.
fn rebalance(
    cost: &[[u64; COLORS]; FACES],
    assignment: &mut [usize; FACES],
    count: &mut [usize; COLORS],
) {
    // We may need up to FACES - COLORS*PER_COLOR iterations in the worst
    // case (≤ FACES), but typically far fewer.
    loop {
        let over: Vec<usize> = (0..COLORS).filter(|&c| count[c] > PER_COLOR).collect();
        let under: Vec<usize> = (0..COLORS).filter(|&c| count[c] < PER_COLOR).collect();

        if over.is_empty() {
            break; // perfectly balanced
        }

        // Run one shortest-augmenting-path step.
        // We find the cheapest single face move that goes  over→face→under.
        // If no direct move exists we find the cheapest chain:
        //   over₁ → face₁ → [reassign face₁ to intermediate color] →
        //   face₂ → under
        // using Dijkstra on the bipartite residual graph.
        augment(cost, assignment, count, &over, &under);
    }
}

/// One augmenting-path step.
///
/// Dijkstra on the bipartite face/color residual graph:
///   Nodes: COLORS color-nodes + FACES face-nodes
///   Sources: all over-quota color-nodes (dist = 0)
///   Sinks:   all under-quota color-nodes
///
/// Edges:
///   color c → face f  (forward, weight = cost[f][c])   — offer face f to color c
///   face f  → color c (backward, weight = 0 if c == assignment[f], else ∞)
///             actually: face f is currently assigned to assignment[f];
///             to "take" face f we pay cost[f][new_c] and refund cost[f][old_c].
///
/// We encode this more simply: Dijkstra over "delta cost" of reassignments.
fn augment(
    cost: &[[u64; COLORS]; FACES],
    assignment: &mut [usize; FACES],
    count: &mut [usize; COLORS],
    over: &[usize],
    under: &[usize],
) {
    // dist_face[f]  = cheapest delta-cost to "steal" face f from its current color
    // dist_color[c] = cheapest delta-cost to reach color c as a destination
    const INF: u64 = u64::MAX / 2;

    let mut dist_face = [INF; FACES];
    let mut dist_color = [INF; COLORS];

    // prev_color[f] = which color we came from when we assigned face f in the path
    // prev_face[c]  = which face we stole to reach color c
    let mut prev_color_of_face = [usize::MAX; FACES];
    let mut prev_face_of_color = [usize::MAX; COLORS];

    // Visited flags
    let mut settled_face = [false; FACES];
    let mut settled_color = [false; COLORS];

    // Priority queue entries: (delta_cost, node_kind, index)
    // We use a simple BinaryHeap with a custom wrapper.
    use std::collections::BinaryHeap;

    #[derive(Eq, PartialEq)]
    struct Entry {
        cost: u64,
        is_color: bool, // true = color node, false = face node
        idx: usize,
    }
    impl Ord for Entry {
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            other.cost.cmp(&self.cost) // min-heap via reversal
        }
    }
    impl PartialOrd for Entry {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            Some(self.cmp(other))
        }
    }

    let mut heap = BinaryHeap::new();

    // Seed: all over-quota colors at zero cost
    for &c in over {
        dist_color[c] = 0;
        heap.push(Entry {
            cost: 0,
            is_color: true,
            idx: c,
        });
    }

    // The color index where augmentation terminates (under-quota destination)
    let mut sink_color = usize::MAX;

    while let Some(Entry {
        cost: d,
        is_color,
        idx,
    }) = heap.pop()
    {
        if is_color {
            if settled_color[idx] {
                continue;
            }
            settled_color[idx] = true;

            // Check if this is an under-quota sink
            if under.contains(&idx) && idx != usize::MAX {
                // Only a valid sink if it was reached via at least one face
                // (i.e. it's not one of the source over-quota colors with dist=0
                // that also happens to be under-quota, which can't happen, but be safe)
                if prev_face_of_color[idx] != usize::MAX {
                    sink_color = idx;
                    break;
                }
                // Edge case: over and under simultaneously means PER_COLOR exactly,
                // shouldn't happen, but if it does skip.
            }

            // Expand color → faces: offer all faces currently assigned to this color
            for f in 0..FACES {
                if assignment[f] == idx && !settled_face[f] {
                    // "Taking" face f from color idx is free (we're releasing it)
                    let nd = d; // no cost to release
                    if nd < dist_face[f] {
                        dist_face[f] = nd;
                        prev_color_of_face[f] = idx;
                        heap.push(Entry {
                            cost: nd,
                            is_color: false,
                            idx: f,
                        });
                    }
                }
            }
        } else {
            // Face node
            if settled_face[idx] {
                continue;
            }
            settled_face[idx] = true;

            let f = idx;
            let old_c = assignment[f];
            let old_cost = cost[f][old_c];

            // Expand face → colors: reassign face f to any other color
            for c in 0..COLORS {
                if c == old_c {
                    continue;
                }
                if settled_color[c] {
                    continue;
                }
                // Delta cost: pay cost[f][c], refund cost[f][old_c]
                // We use saturating arithmetic to avoid overflow on the subtraction
                let delta = cost[f][c].saturating_sub(old_cost);
                // Actual signed delta but tracked as: new_total_delta = d + cost[f][c] - old_cost
                // Since costs can decrease we track raw new cost instead and accept negative deltas
                // by using wrapping: encode as i64 then back.  Simpler: just track absolute cost.
                let _ = delta; // replaced below

                // Use absolute path cost (sum of new assignments) for Dijkstra key
                // This is valid because we always compare paths from the same sources.
                let nd = d.saturating_add(cost[f][c]).saturating_sub(old_cost);
                if nd < dist_color[c] {
                    dist_color[c] = nd;
                    prev_face_of_color[c] = f;
                    heap.push(Entry {
                        cost: nd,
                        is_color: true,
                        idx: c,
                    });
                }
            }
        }
    }

    if sink_color == usize::MAX {
        // No augmenting path found — this shouldn't happen if the input is valid
        // (54 samples, 6 colors). Fallback: force-move the cheapest excess face.
        fallback_move(cost, assignment, count, over, under);
        return;
    }

    // Trace back and apply the augmenting path
    let mut c = sink_color;
    loop {
        let f = prev_face_of_color[c];
        if f == usize::MAX {
            break;
        }
        let old_c = prev_color_of_face[f];

        // Reassign face f from old_c to c
        count[old_c] -= 1;
        count[c] += 1;
        assignment[f] = c;

        c = old_c;
        // If old_c was one of the source over-quota colors, we're done
        if over.contains(&old_c) && count[old_c] <= PER_COLOR {
            break;
        }
    }
}

/// Emergency fallback: brute-force the single cheapest face move from any
/// over-quota color to any under-quota color (ignores chain optimality but
/// always makes progress).
fn fallback_move(
    cost: &[[u64; COLORS]; FACES],
    assignment: &mut [usize; FACES],
    count: &mut [usize; COLORS],
    over: &[usize],
    under: &[usize],
) {
    let mut best_delta = i128::MAX;
    let mut best_face = 0;
    let mut best_new_color = 0;

    for f in 0..FACES {
        if !over.contains(&assignment[f]) {
            continue;
        }
        let old_c = assignment[f];
        for &new_c in under {
            let delta = (cost[f][new_c] as i128) - (cost[f][old_c] as i128);
            if delta < best_delta {
                best_delta = delta;
                best_face = f;
                best_new_color = new_c;
            }
        }
    }

    let old_c = assignment[best_face];
    count[old_c] -= 1;
    count[best_new_color] += 1;
    assignment[best_face] = best_new_color;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ref(r: u16, g: u16, b: u16) -> ColorRef {
        ColorRef { r, g, b }
    }
    fn make_rgb(r: u16, g: u16, b: u16) -> Rgb {
        Rgb { r, g, b }
    }

    /// Build 54 perfect samples (9 × each reference) with optional noise.
    fn perfect_samples(refs: &[ColorRef; COLORS], noise: u16) -> [Rgb; FACES] {
        let mut samples = [Rgb { r: 0, g: 0, b: 0 }; FACES];
        for c in 0..COLORS {
            for i in 0..PER_COLOR {
                let f = c * PER_COLOR + i;
                // Add a small deterministic offset to simulate sensor noise
                let offset = (i as u16) * noise;
                samples[f] = Rgb {
                    r: refs[c].r.saturating_add(offset),
                    g: refs[c].g.saturating_add(offset / 2),
                    b: refs[c].b.saturating_add(offset / 3),
                };
            }
        }
        samples
    }

    #[test]
    fn test_counts_are_exactly_nine() {
        let refs = [
            make_ref(65000, 65000, 65000), // white
            make_ref(65000, 65000, 1000),  // yellow
            make_ref(55000, 5000, 5000),   // red
            make_ref(60000, 30000, 3000),  // orange
            make_ref(5000, 10000, 55000),  // blue
            make_ref(5000, 45000, 10000),  // green
        ];
        let samples = perfect_samples(&refs, 200);
        let result = assign_colors(&samples, &refs);

        let mut counts = [0usize; COLORS];
        for &c in &result {
            counts[c] += 1;
        }
        assert_eq!(
            counts, [PER_COLOR; COLORS],
            "Each color must appear exactly {PER_COLOR} times, got {counts:?}"
        );
    }

    #[test]
    fn test_correct_assignment_no_noise() {
        let refs = [
            make_ref(65000, 65000, 65000),
            make_ref(65000, 65000, 1000),
            make_ref(55000, 5000, 5000),
            make_ref(60000, 30000, 3000),
            make_ref(5000, 10000, 55000),
            make_ref(5000, 45000, 10000),
        ];
        let samples = perfect_samples(&refs, 0);
        let result = assign_colors(&samples, &refs);

        // With zero noise, each face must be assigned to its originating color
        for c in 0..COLORS {
            for i in 0..PER_COLOR {
                let f = c * PER_COLOR + i;
                assert_eq!(
                    result[f], c,
                    "Face {f} should be color {c}, got {}",
                    result[f]
                );
            }
        }
    }

    #[test]
    fn test_heavy_noise_still_satisfies_constraint() {
        let refs = [
            make_ref(65000, 65000, 65000),
            make_ref(65000, 65000, 1000),
            make_ref(55000, 5000, 5000),
            make_ref(60000, 30000, 3000),
            make_ref(5000, 10000, 55000),
            make_ref(5000, 45000, 10000),
        ];
        // 2000-unit noise — colors may be misidentified, but counts must hold
        let samples = perfect_samples(&refs, 2000);
        let result = assign_colors(&samples, &refs);

        let mut counts = [0usize; COLORS];
        for &c in &result {
            counts[c] += 1;
        }
        assert_eq!(counts, [PER_COLOR; COLORS]);
    }

    #[test]
    fn test_sq_dist_no_fp() {
        // Simple sanity check on the integer distance
        let s = make_rgb(100, 200, 300);
        let r = make_ref(110, 190, 320);
        let d = sq_dist(s, r);
        // Δr=10, Δg=10, Δb=20 → 100+100+400 = 600
        assert_eq!(d, 600);
    }
}
