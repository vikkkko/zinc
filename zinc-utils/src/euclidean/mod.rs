//!
//! The Euclidean division and remainder.
//!

#[cfg(test)]
mod tests;

use std::ops::Div;

use num::BigInt;
use num::One;
use num::Signed;
use num::Zero;

///
/// Euclidean division of BigInt.
///
/// div_rem(9, 4) -> (2, 1)
/// div_rem(9, -4) -> (-2, 1)
/// div_rem(-9, 4) -> (-3, 3)
/// div_rem(-9, -4) -> (3, 3)
pub fn div_rem(nominator: &BigInt, denominator: &BigInt) -> Option<(BigInt, BigInt)> {
    if denominator.is_zero() {
        return None;
    }

    let mut div = nominator.div(denominator);

    if div.clone() * denominator.clone() > nominator.clone() {
        if denominator.is_positive() {
            div -= BigInt::one();
        } else {
            div += BigInt::one();
        }
    }

    let rem = nominator - div.clone() * denominator;

    Some((div, rem))
}

#[cfg(test)]
mod test {
    use num::BigInt;

    use crate::euclidean;

    #[test]
    fn test_div_rem() {
        let (d, r) = euclidean::div_rem(&BigInt::from(9), &BigInt::from(4))
            .expect(zinc_const::panic::TEST_DATA_VALID);
        assert_eq!(d, BigInt::from(2));
        assert_eq!(r, BigInt::from(1));

        let (d, r) = euclidean::div_rem(&BigInt::from(-9), &BigInt::from(-4))
            .expect(zinc_const::panic::TEST_DATA_VALID);
        assert_eq!(d, BigInt::from(3));
        assert_eq!(r, BigInt::from(3));

        let (d, r) = euclidean::div_rem(&BigInt::from(-9), &BigInt::from(4))
            .expect(zinc_const::panic::TEST_DATA_VALID);
        assert_eq!(d, BigInt::from(-3));
        assert_eq!(r, BigInt::from(3));

        let (d, r) = euclidean::div_rem(&BigInt::from(9), &BigInt::from(-4))
            .expect(zinc_const::panic::TEST_DATA_VALID);
        assert_eq!(d, BigInt::from(-2));
        assert_eq!(r, BigInt::from(1));
    }
}