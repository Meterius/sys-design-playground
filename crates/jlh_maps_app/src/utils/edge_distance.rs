use smallvec::SmallVec;

#[derive(Clone, Copy)]
struct Edge {
    ax: f32,
    ay: f32,
    bx: f32,
    by: f32,
    dx: f32,
    dy: f32,
    len_sq: f32,
}

pub fn update_edge_distance_texture(
    edges: &[f32],
    out: &mut [f32],
    texture_width: usize,
    texture_height: usize,
    max_distance: f32,
) {
    if texture_width == 0 || texture_height == 0 || max_distance <= 0.0 {
        return;
    }

    let expected_len = texture_width.saturating_mul(texture_height);
    if out.len() < expected_len {
        return;
    }

    let edges = prepare_edges(edges);
    if edges.is_empty() {
        out[..expected_len].fill(0.0);
        return;
    }

    let bins = EdgeBins::new(&edges, max_distance);
    let max_dist_sq = max_distance * max_distance;

    for y in 0..texture_height {
        let row_offset = y * texture_width;
        let py = (y as f32 + 0.5) / texture_height as f32;

        for x in 0..texture_width {
            let px = (x as f32 + 0.5) / texture_width as f32;
            let mut closest_dist_sq = max_dist_sq;

            for edge in bins.candidates(px, py) {
                let dist_sq = distance_to_edge_sq(px, py, edge);
                if dist_sq < closest_dist_sq {
                    closest_dist_sq = dist_sq;
                }
            }

            let distance = closest_dist_sq.sqrt().min(max_distance);
            out[row_offset + x] = distance / max_distance;
        }
    }
}

fn prepare_edges(edges: &[f32]) -> Vec<Edge> {
    edges
        .chunks_exact(4)
        .filter_map(|edge| {
            let dx = edge[2] - edge[0];
            let dy = edge[3] - edge[1];
            let len_sq = dx * dx + dy * dy;

            if len_sq <= f32::EPSILON {
                return None;
            }

            Some(Edge {
                ax: edge[0],
                ay: edge[1],
                bx: edge[2],
                by: edge[3],
                dx,
                dy,
                len_sq,
            })
        })
        .collect()
}

struct EdgeBins<'a> {
    bin_count: usize,
    bins: Vec<SmallVec<[&'a Edge; 5]>>,
}

impl<'a> EdgeBins<'a> {
    fn new(edges: &'a [Edge], max_distance: f32) -> Self {
        let bin_count = (1.0 / max_distance).ceil().clamp(1.0, 256.0) as usize;
        let mut bins = vec![SmallVec::<[&Edge; 5]>::new(); bin_count * bin_count];

        for edge in edges {
            let min_x = clamp_bin(
                ((edge.ax.min(edge.bx) - max_distance) * bin_count as f32).floor(),
                bin_count,
            );
            let max_x = clamp_bin(
                ((edge.ax.max(edge.bx) + max_distance) * bin_count as f32).floor(),
                bin_count,
            );
            let min_y = clamp_bin(
                ((edge.ay.min(edge.by) - max_distance) * bin_count as f32).floor(),
                bin_count,
            );
            let max_y = clamp_bin(
                ((edge.ay.max(edge.by) + max_distance) * bin_count as f32).floor(),
                bin_count,
            );

            for y in min_y..=max_y {
                let row_offset = y * bin_count;
                for x in min_x..=max_x {
                    bins[row_offset + x].push(edge);
                }
            }
        }

        Self { bin_count, bins }
    }

    fn candidates(&self, x: f32, y: f32) -> &[&'a Edge] {
        let bin_x = clamp_bin((x * self.bin_count as f32).floor(), self.bin_count);
        let bin_y = clamp_bin((y * self.bin_count as f32).floor(), self.bin_count);

        self.bins[bin_y * self.bin_count + bin_x].as_slice()
    }
}

fn clamp_bin(value: f32, bin_count: usize) -> usize {
    value.max(0.0).min((bin_count - 1) as f32) as usize
}

fn distance_to_edge_sq(px: f32, py: f32, edge: &Edge) -> f32 {
    let t = (((px - edge.ax) * edge.dx + (py - edge.ay) * edge.dy) / edge.len_sq).clamp(0.0, 1.0);
    let closest_x = edge.ax + t * edge.dx;
    let closest_y = edge.ay + t * edge.dy;
    let dist_x = px - closest_x;
    let dist_y = py - closest_y;

    dist_x * dist_x + dist_y * dist_y
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binned_distance_matches_bruteforce() {
        let raw_edges = [
            0.1, 0.1, 0.9, 0.1, //
            0.2, 0.2, 0.8, 0.8, //
            0.05, 0.95, 0.95, 0.45,
        ];
        let texture_width = 32;
        let texture_height = 24;
        let max_distance = 0.15;
        let mut actual = vec![0.0; texture_width * texture_height];
        let expected = brute_force(&raw_edges, texture_width, texture_height, max_distance);

        update_edge_distance_texture(
            &raw_edges,
            &mut actual,
            texture_width,
            texture_height,
            max_distance,
        );

        assert_eq!(actual.len(), expected.len());
        for (actual, expected) in actual.iter().zip(expected.iter()) {
            assert!((actual - expected).abs() < f32::EPSILON);
        }
    }

    fn brute_force(
        raw_edges: &[f32],
        texture_width: usize,
        texture_height: usize,
        max_distance: f32,
    ) -> Vec<f32> {
        let edges = prepare_edges(raw_edges);
        let max_dist_sq = max_distance * max_distance;
        let mut out = vec![0.0; texture_width * texture_height];

        for y in 0..texture_height {
            let row_offset = y * texture_width;
            let py = (y as f32 + 0.5) / texture_height as f32;

            for x in 0..texture_width {
                let px = (x as f32 + 0.5) / texture_width as f32;
                let mut closest_dist_sq = max_dist_sq;

                for edge in &edges {
                    let dist_sq = distance_to_edge_sq(px, py, edge);
                    if dist_sq < closest_dist_sq {
                        closest_dist_sq = dist_sq;
                    }
                }

                let distance = closest_dist_sq.sqrt().min(max_distance);
                out[row_offset + x] = distance / max_distance;
            }
        }

        out
    }
}
