use std::ops::Mul;

#[derive(Debug, PartialEq)]
pub struct Matrix {
    rows: usize,
    cols: usize,
    values: Vec<f32>,
}

impl Matrix {
    pub fn new(rows: usize, cols: usize, values: Vec<f32>) -> Self {
        Self { rows, cols, values }
    }

    pub fn shape(&self) -> (usize, usize) {
        (self.rows, self.cols)
    }
}

impl Mul for &Matrix {
    type Output = Matrix;

    fn mul(self, rhs: &Matrix) -> Self::Output {
        let lhs: &Matrix = self;
        let (lhs_rows, lhs_cols) = lhs.shape();
        let (rhs_rows, rhs_cols) = rhs.shape();
        let (output_rows, output_cols) = (rhs_rows, lhs_cols);
        let mut values = Vec::new();
        values.resize(output_rows * output_cols, 0.0);

        let mut output = Matrix::new(output_rows, output_cols, values);
        let mut output_index: usize = 0;
        let lhs_stride = lhs_cols;

        for row in 0..output_rows {
            for col in 0..output_cols {
                let mut lhs_index = col;
                let mut rhs_index = row * rhs_cols;
                let mut output_value = 0.0;
                for _ in 0..lhs_rows {
                    output_value += lhs.values[lhs_index] * rhs.values[rhs_index];
                    lhs_index += lhs_stride;
                    rhs_index += 1;
                }
                output.values[output_index] = output_value;
                output_index += 1;
            }
        }
        output
    }
}

#[cfg(test)]
mod tests {
    use crate::matrix::Matrix;

    #[test]
    fn new() {
        // Given rows and cols
        // When a matrix is built
        // Then it has the appropriate shape

        let matrix = Matrix::new(
            4,
            3,
            vec![
                //
                0.0, 0.0, 0.0, //
                0.0, 0.0, 0.0, //
                0.0, 0.0, 0.0, //
                0.0, 0.0, 0.0, //
            ],
        );
        assert_eq!(matrix.shape(), (4, 3));
    }

    // TODO add a test for incorrect shapes in multiplication

    #[test]
    fn multiplication() {
        // Given a left-hand side matrix and and a right-hand side matrix
        // When the multiplication lhs * rhs is done
        // Then the resulting matrix has the correct values

        let lhs = Matrix::new(
            3,
            2,
            vec![
                //
                1.0, 2.0, //
                3.0, 4.0, //
                5.0, 6.0, //
            ],
        );
        let rhs = Matrix::new(
            2,
            3,
            vec![
                //
                11.0, 12.0, 13.0, //
                14.0, 15.0, 16.0, //
            ],
        );
        let actual_product = &lhs * &rhs;
        let expected_product = Matrix::new(
            2,
            2,
            vec![
                //
                1.0 * 11.0 + 3.0 * 12.0 + 5.0 * 13.0,
                2.0 * 11.0 + 4.0 * 12.0 + 6.0 * 13.0, //
                1.0 * 14.0 + 3.0 * 15.0 + 5.0 * 16.0,
                2.0 * 14.0 + 4.0 * 15.0 + 6.0 * 16.0, //
            ],
        );

        assert_eq!(actual_product, expected_product);
    }
}
