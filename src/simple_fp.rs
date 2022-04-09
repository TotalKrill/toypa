use std::num::ParseFloatError;

use serde::Serializer;
use std::ops::{Add, AddAssign, Sub, SubAssign};

#[derive(Debug, PartialOrd, Copy, Clone, PartialEq, Eq, Ord)]
/// Fixed point implementation for numbers with a 4 decimals point, achieved by instead of storing
/// Decimal numbers, only allow operations on integers representing TenThoushanth's
pub struct FixedPoint(i128);

impl FixedPoint {
    // Yep, I did this, sue me (please dont)
    pub fn from_f64(n: f64) -> Self {
        let s = format!("{:0.4}", n);
        let s = s.replace(".", "");
        let n: i128 = s.parse().unwrap();
        Self(n)
    }
    pub fn from_f32(n: f32) -> Self {
        let s = format!("{:0.4}", n);
        let s = s.replace(".", "");
        let n: i128 = s.parse().unwrap();
        Self(n)
    }
    pub fn to_f32(self) -> f32 {
        let f = self.0 as f32;
        let f = f * 1000.0;
        f
    }
}

impl PartialEq<FixedPoint> for f64 {
    fn eq(&self, other: &FixedPoint) -> bool {
        FixedPoint::from_f64(*self) == *other
    }
}
impl PartialEq<FixedPoint> for f32 {
    fn eq(&self, other: &FixedPoint) -> bool {
        FixedPoint::from_f32(*self) == *other
    }
}

impl PartialEq<f64> for FixedPoint {
    fn eq(&self, other: &f64) -> bool {
        FixedPoint::from_f64(*other) == *self
    }
}
impl PartialEq<f32> for FixedPoint {
    fn eq(&self, other: &f32) -> bool {
        FixedPoint::from_f32(*other) == *self
    }
}

impl Add for FixedPoint {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}
impl AddAssign for FixedPoint {
    fn add_assign(&mut self, rhs: Self) {
        self.0 = self.0 + rhs.0;
    }
}
impl SubAssign for FixedPoint {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 = self.0 - rhs.0;
    }
}

impl Sub for FixedPoint {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn fixedpoint_tests() {
        let v = 0.12340;
        let fp = FixedPoint::from_f64(v);

        assert_eq!(1234, fp.0);
    }
}
