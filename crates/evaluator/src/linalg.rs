use crate::value::Matrix;
use crate::EvalError;
use nalgebra::{DMatrix, DVector, SymmetricEigen};

// CONVENTION: crate::value::Matrix stores data ROW-major.
// nalgebra stores data COLUMN-major. Every conversion must go through
// to_dm / from_dm below — never use DMatrix::iter() to build our Matrix.

pub fn to_dm(m: &Matrix) -> DMatrix<f64> {
    DMatrix::from_row_slice(m.rows, m.cols, &m.data)
}

pub fn from_dm(d: &DMatrix<f64>) -> Matrix {
    let mut data = Vec::with_capacity(d.nrows() * d.ncols());
    for r in 0..d.nrows() {
        for c in 0..d.ncols() {
            data.push(d[(r, c)]);
        }
    }
    Matrix::new(d.nrows(), d.ncols(), data)
}

pub fn matmul(a: &Matrix, b: &Matrix) -> Result<Matrix, EvalError> {
    if a.cols != b.rows {
        return Err(EvalError::Args(format!(
            "cannot multiply {}x{} by {}x{}", a.rows, a.cols, b.rows, b.cols
        )));
    }
    Ok(from_dm(&(to_dm(a) * to_dm(b))))
}

pub fn matvec(a: &Matrix, v: &[f64]) -> Result<Vec<f64>, EvalError> {
    if a.cols != v.len() {
        return Err(EvalError::Args(format!(
            "cannot multiply {}x{} matrix by vector of length {}", a.rows, a.cols, v.len()
        )));
    }
    let r = to_dm(a) * DVector::from_column_slice(v);
    // Single column: column-major iteration equals row order.
    Ok(r.iter().copied().collect())
}

pub fn transpose(m: &Matrix) -> Matrix {
    from_dm(&to_dm(m).transpose())
}

pub fn det(m: &Matrix) -> Result<f64, EvalError> {
    if m.rows != m.cols {
        return Err(EvalError::Args("determinant requires a square matrix".into()));
    }
    Ok(to_dm(m).determinant())
}

pub fn inv(m: &Matrix) -> Result<Matrix, EvalError> {
    if m.rows != m.cols {
        return Err(EvalError::Args("inverse requires a square matrix".into()));
    }
    match to_dm(m).try_inverse() {
        Some(i) => Ok(from_dm(&i)),
        None => Err(EvalError::Math("matrix is singular".into())),
    }
}

/// Rank via singular values (avoids decomposition-specific API differences).
pub fn rank(m: &Matrix) -> Result<usize, EvalError> {
    let svd = to_dm(m).svd(false, false);
    Ok(svd.singular_values.iter().filter(|v| **v > 1e-10).count())
}

pub fn solve(a: &Matrix, b: &Matrix) -> Result<Matrix, EvalError> {
    if a.rows != a.cols {
        return Err(EvalError::Args("solve requires a square coefficient matrix".into()));
    }
    let lu = to_dm(a).lu();
    match lu.solve(&to_dm(b)) {
        Some(x) => Ok(from_dm(&x)),
        None => Err(EvalError::Math("linear system has no unique solution".into())),
    }
}

/// Symmetric eigenproblem. General matrices are delegated to NumPy.
pub fn eigen_symmetric(m: &Matrix) -> Result<(Vec<f64>, Matrix), EvalError> {
    if m.rows != m.cols {
        return Err(EvalError::Args("eigenvalues require a square matrix".into()));
    }
    let dm = to_dm(m);
    let asym = (&dm - dm.transpose()).iter().fold(0.0f64, |acc, v| acc.max(v.abs()));
    if asym > 1e-8 {
        return Err(EvalError::RequiresPython("eigenvalues of a non-symmetric matrix".into()));
    }
    let eig = SymmetricEigen::new(dm);
    Ok((eig.eigenvalues.iter().copied().collect(), from_dm(&eig.eigenvectors)))
}

pub fn polyfit(x: &[f64], y: &[f64], degree: usize) -> Result<Vec<f64>, EvalError> {
    if x.len() != y.len() || x.len() < degree + 1 {
        return Err(EvalError::Args(
            "polyfit needs matching x/y arrays with at least degree+1 points".into(),
        ));
    }
    let n = x.len();
    let mut a = DMatrix::<f64>::zeros(n, degree + 1);
    for i in 0..n {
        for j in 0..=degree {
            a[(i, j)] = x[i].powi((degree - j) as i32);
        }
    }
    let b = DVector::from_column_slice(y);
    let svd = a.svd(true, true);
    match svd.solve(&b, 1e-12) {
        // Solution is a single column vector: iteration order is unambiguous.
        Ok(sol) => Ok(sol.iter().copied().collect()),
        Err(e) => Err(EvalError::Math(format!("polyfit failed: {e}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversions_preserve_row_major_order() {
        let m = Matrix::new(2, 3, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        let back = from_dm(&to_dm(&m));
        assert_eq!(back.data, m.data);
    }

    #[test]
    fn matmul_matches_hand_calculation() {
        let a = Matrix::new(2, 2, vec![1.0, 2.0, 3.0, 4.0]);
        let b = Matrix::new(2, 2, vec![5.0, 6.0, 7.0, 8.0]);
        assert_eq!(matmul(&a, &b).unwrap().data, vec![19.0, 22.0, 43.0, 50.0]);
    }

    #[test]
    fn transpose_swaps_rows_and_columns() {
        let m = Matrix::new(2, 3, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        let t = transpose(&m);
        assert_eq!((t.rows, t.cols), (3, 2));
        assert_eq!(t.data, vec![1.0, 4.0, 2.0, 5.0, 3.0, 6.0]);
    }

    #[test]
    fn inverse_round_trip_is_identity() {
        let a = Matrix::new(2, 2, vec![4.0, 7.0, 2.0, 6.0]);
        let product = matmul(&a, &inv(&a).unwrap()).unwrap();
        for (i, v) in product.data.iter().enumerate() {
            let expected = if i == 0 || i == 3 { 1.0 } else { 0.0 };
            assert!((v - expected).abs() < 1e-9, "element {i} = {v}");
        }
    }

    #[test]
    fn solve_matches_known_solution() {
        // 2x + y = 5 ; x + 3y = 10  ->  x = 1, y = 3
        let a = Matrix::new(2, 2, vec![2.0, 1.0, 1.0, 3.0]);
        let b = Matrix::new(2, 1, vec![5.0, 10.0]);
        let x = solve(&a, &b).unwrap();
        assert!((x.data[0] - 1.0).abs() < 1e-9);
        assert!((x.data[1] - 3.0).abs() < 1e-9);
    }
}
