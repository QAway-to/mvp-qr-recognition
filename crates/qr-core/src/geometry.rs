use image::{GrayImage, ImageBuffer, Luma};
use nalgebra::{Matrix3, Point2, Vector3};

/// Apply perspective warp to an image
pub fn warp_perspective(
    img: &GrayImage,
    matrix: &Matrix3<f32>,
    out_width: u32,
    out_height: u32,
) -> GrayImage {
    let mut output = ImageBuffer::new(out_width, out_height);
    let inv_matrix = matrix.try_inverse().unwrap_or(Matrix3::identity());

    for y in 0..out_height {
        for x in 0..out_width {
            // Map output pixel (x, y) back to source image
            let dst_point = Vector3::new(x as f32, y as f32, 1.0);
            let src_point_h = inv_matrix * dst_point;
            
            // Normalize homogeneous coordinates
            let z = src_point_h.z;
            if z.abs() < 1e-6 {
                continue;
            }
            
            let src_x = src_point_h.x / z;
            let src_y = src_point_h.y / z;

            // Bilinear interpolation
            let pixel = bilinear_sample(img, src_x, src_y);
            output.put_pixel(x, y, Luma([pixel]));
        }
    }
    output
}

/// Compute Homography Matrix mapping src_points to dst_points
/// Uses 4 corresponding points.
pub fn find_homography(
    src: [Point2<f32>; 4],
    dst: [Point2<f32>; 4],
) -> Option<Matrix3<f32>> {
    // Basic Direct Linear Transform (DLT) solver
    // For 4 points, we solve Ah = 0.
    
    // Using nalgebra's DMatrix for solving SVD or linear system would be ideal,
    // but for fixed 4 points we can just construct the 8x9 matrix.
    // However, opencv "getPerspectiveTransform" is robust.
    
    // We can use a simpler approach if we trust the points form a convex quad.
    // But let's implementing a Gaussian elimination or SVD based solver for Ax=0 is heavy.
    // Better idea: map unit square to quad, or quad to quad.
    
    // Let's implement the standard 8-mult-eqn solver.
    let mut matrix_a = nalgebra::DMatrix::<f32>::zeros(8, 9);
    
    for i in 0..4 {
        let x = src[i].x;
        let y = src[i].y;
        let u = dst[i].x;
        let v = dst[i].y;
        
        matrix_a[(i * 2, 0)] = -x;
        matrix_a[(i * 2, 1)] = -y;
        matrix_a[(i * 2, 2)] = -1.0;
        matrix_a[(i * 2, 3)] = 0.0;
        matrix_a[(i * 2, 4)] = 0.0;
        matrix_a[(i * 2, 5)] = 0.0;
        matrix_a[(i * 2, 6)] = x * u;
        matrix_a[(i * 2, 7)] = y * u;
        matrix_a[(i * 2, 8)] = u;

        matrix_a[(i * 2 + 1, 0)] = 0.0;
        matrix_a[(i * 2 + 1, 1)] = 0.0;
        matrix_a[(i * 2 + 1, 2)] = 0.0;
        matrix_a[(i * 2 + 1, 3)] = -x;
        matrix_a[(i * 2 + 1, 4)] = -y;
        matrix_a[(i * 2 + 1, 5)] = -1.0;
        matrix_a[(i * 2 + 1, 6)] = x * v;
        matrix_a[(i * 2 + 1, 7)] = y * v;
        matrix_a[(i * 2 + 1, 8)] = v;
    }

    // Solve using SVD
    let svd = matrix_a.svd(false, true);
    if let Some(v_t) = svd.v_t {
         // Safety check: ensure we have enough rows
         if v_t.nrows() < 9 {
             return None;
         }
         
         // The solution is the last row of V^T (or last column of V) corresponding to smallest singular value.
         // svd.v_t is V^T. The last row correspond to smallest sigma.
         let h_vec = v_t.row(8);
         
         let h = Matrix3::new(
             h_vec[0], h_vec[1], h_vec[2],
             h_vec[3], h_vec[4], h_vec[5],
             h_vec[6], h_vec[7], h_vec[8]
         );
         
         // Normalize so h[8] is 1 (if not zero)
         if h[8].abs() > 1e-6 {
             return Some(h / h[8]);
         }
         return Some(h);
    }
    
    None
}

fn bilinear_sample(img: &GrayImage, x: f32, y: f32) -> u8 {
    let width = img.width() as f32;
    let height = img.height() as f32;
    
    if x < 0.0 || x >= width - 1.0 || y < 0.0 || y >= height - 1.0 {
        return 0; // Black padding
    }
    
    let x0 = x.floor() as u32;
    let y0 = y.floor() as u32;
    let x1 = x0 + 1;
    let y1 = y0 + 1;
    
    let dx = x - x0 as f32;
    let dy = y - y0 as f32;
    
    let p00 = img.get_pixel(x0, y0).0[0] as f32;
    let p10 = img.get_pixel(x1, y0).0[0] as f32;
    let p01 = img.get_pixel(x0, y1).0[0] as f32;
    let p11 = img.get_pixel(x1, y1).0[0] as f32;
    
    let top = p00 * (1.0 - dx) + p10 * dx;
    let bottom = p01 * (1.0 - dx) + p11 * dx;
    
    (top * (1.0 - dy) + bottom * dy) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_homography_integrity() {
        let src = [
            Point2::new(0.0, 0.0),
            Point2::new(10.0, 0.0),
            Point2::new(10.0, 10.0),
            Point2::new(0.0, 10.0),
        ];
        let dst = src.clone();
        
        let h = find_homography(src, dst).unwrap();
        // Should be roughly identity
        assert!((h[(0,0)] - 1.0).abs() < 1e-3);
    }
}
