use super::Bint;
use crate::arithmetic;
use crate::digit::{SignedDigit, Digit};
use crate::macros::{overflowing_pow, div_zero, rem_zero, op_ref_impl};
use crate::{ExpType, BUint};
use crate::digit;

impl<const N: usize> Bint<N> {
    #[inline]
    pub const fn overflowing_add(self, rhs: Self) -> (Self, bool) {
        let mut digits = [0; N];
        let mut carry = 0u8;

        let self_digits = self.digits();
        let rhs_digits = rhs.digits();

        let mut i = 0;
        while i < Self::N_MINUS_1 {
            let (sum, c) = arithmetic::add_carry_unsigned(carry, self_digits[i], rhs_digits[i]);
            digits[i] = sum;
            carry = c;
            i += 1;
        }
        let (sum, carry) = arithmetic::add_carry_signed(
            carry,
            self_digits[Self::N_MINUS_1] as SignedDigit,
            rhs_digits[Self::N_MINUS_1] as SignedDigit
        );
        digits[Self::N_MINUS_1] = sum as Digit;

        (Self::from_digits(digits), carry)
    }

    #[inline]
    pub const fn overflowing_add_unsigned(self, rhs: BUint<N>) -> (Self, bool) {
        let rhs = Self::from_bits(rhs);
        let (out, overflow) = self.overflowing_add(rhs);
        (out, overflow ^ rhs.is_negative())
    }

    #[inline]
    pub const fn overflowing_sub(self, rhs: Self) -> (Self, bool) {
        let mut digits = [0; N];
        let mut borrow = 0u8;

        let self_digits = self.digits();
        let rhs_digits = rhs.digits();

        let mut i = 0;
        while i < Self::N_MINUS_1 {
            let (sub, b) = arithmetic::sub_borrow_unsigned(borrow, self_digits[i], rhs_digits[i]);
            digits[i] = sub;
            borrow = b;
            i += 1;
        }
        let (sub, borrow) = arithmetic::sub_borrow_signed(
            borrow,
            self_digits[Self::N_MINUS_1] as SignedDigit,
            rhs_digits[Self::N_MINUS_1] as SignedDigit
        );
        digits[Self::N_MINUS_1] = sub as Digit;

        (Self::from_digits(digits), borrow)
    }

    #[inline]
    pub const fn overflowing_sub_unsigned(self, rhs: BUint<N>) -> (Self, bool) {
        let rhs = Self::from_bits(rhs);
        let (out, overflow) = self.overflowing_sub(rhs);
        (out, overflow ^ rhs.is_negative())
    }
    
    #[inline]
    pub const fn overflowing_mul(self, rhs: Self) -> (Self, bool) {
        let (uint, overflow) = self.unsigned_abs().overflowing_mul(rhs.unsigned_abs());
        let out = Self {
            uint,
        };
        if self.is_negative() == rhs.is_negative() {
            (out, overflow || out.is_negative())
        } else {
            match out.checked_neg() {
                Some(n) => (n, overflow || out.is_negative()),
                None => (out, overflow),
            }
        }
    }
    #[inline]
    const fn div_rem_unchecked(self, rhs: Self) -> (Self, Self) {
        let (div, rem) = self.unsigned_abs().div_rem_unchecked(rhs.unsigned_abs());
        let div = Self {
            uint: div,
        };
        let rem = Self {
            uint: rem,
        };
        if self.is_negative() {
            if rhs.is_negative() {
                (div, rem.neg())
            } else {
                (div.neg(), rem.neg())
            }
        } else if rhs.is_negative() {
            (div.neg(), rem)
        } else {
            (div, rem)
        }
    }
    #[inline]
    pub const fn overflowing_div(self, rhs: Self) -> (Self, bool) {
        if self.eq(&Self::MIN) && rhs.eq(&Self::NEG_ONE) {
            (self, true)
        } else {
            if rhs.is_zero() {
                div_zero!()
            }
            (self.div_rem_unchecked(rhs).0, false)
        }
    }
    #[inline]
    pub const fn overflowing_div_euclid(self, rhs: Self) -> (Self, bool) {
        if self.eq(&Self::MIN) && rhs.eq(&Self::NEG_ONE) {
            (self, true)
        } else {
            if rhs.is_zero() {
                div_zero!()
            }
            let (div, rem) = self.div_rem_unchecked(rhs);
            if self.is_negative() {
                if rem.is_zero() {
                    (div, false)
                } else {
                    let div = if rhs.is_negative() {
                        div.add(Self::ONE)
                    } else {
                        div.sub(Self::ONE)
                    };
                    (div, false)
                }
            } else {
                (div, false)
            }
        }
    }
    #[inline]
    pub const fn overflowing_rem(self, rhs: Self) -> (Self, bool) {
        if self.eq(&Self::MIN) && rhs.eq(&Self::NEG_ONE) {
            (Self::ZERO, true)
        } else {
            if rhs.is_zero() {
                div_zero!()
            }
            (self.div_rem_unchecked(rhs).1, false)
        }
    }
    #[inline]
    pub const fn overflowing_rem_euclid(self, rhs: Self) -> (Self, bool) {
        if self.eq(&Self::MIN) && rhs.eq(&Self::NEG_ONE) {
            (Self::ZERO, true)
        } else {
            if rhs.is_zero() {
                rem_zero!()
            }
            let rem = self.div_rem_unchecked(rhs).1;
            if rem.is_negative() {
                let r = rem.add(rhs.abs());
                (r, false)
            } else {
                (rem, false)
            }
        }
    }
    #[inline]
    pub const fn overflowing_neg(self) -> (Self, bool) {
        (!self).overflowing_add(Self::ONE)
    }
    #[inline]
    pub const fn overflowing_shl(self, rhs: ExpType) -> (Self, bool) {
        let (uint, overflow) = self.uint.overflowing_shl(rhs);
        (Self { uint }, overflow)
    }
    #[inline]
    const fn shr_internal(self, rhs: ExpType) -> Self {
        if rhs == 0 {
            self
        } else {
            let digit_shift = (rhs >> digit::BIT_SHIFT) as usize;
            let shift = (rhs & digit::BITS_MINUS_1) as u8;
    
            let mut out_digits = [Digit::MAX; N];
            let digits_ptr = self.digits().as_ptr();
            let out_ptr = out_digits.as_mut_ptr() as *mut Digit;
            unsafe {
                digits_ptr.add(digit_shift).copy_to_nonoverlapping(out_ptr, N - digit_shift);
                core::mem::forget(self);
            }
    
            if shift > 0 {
                let mut borrow = 0;
                let borrow_shift = Digit::BITS as u8 - shift;

                let mut i = digit_shift;
                while i < N {
                    let digit = out_digits[Self::N_MINUS_1 - i];
                    let new_borrow = digit << borrow_shift;
                    let new_digit = (digit >> shift) | borrow;
                    out_digits[Self::N_MINUS_1 - i] = new_digit;
                    borrow = new_borrow;
                    i += 1;
                }
                out_digits[Self::N_MINUS_1 - digit_shift] |= (Digit::MAX >> (digit::BITS as u8 - shift)) << (digit::BITS as u8 - shift);
            }
    
            Self::from_digits(out_digits)
        }
    }
    const BITS_MINUS_1: ExpType = (Self::BITS - 1) as ExpType;
    #[inline]
    pub const fn overflowing_shr(self, rhs: ExpType) -> (Self, bool) {
        if self.is_negative() {
            if rhs >= Self::BITS {
                (self.shr_internal(rhs & Self::BITS_MINUS_1), true)
            } else {
                (self.shr_internal(rhs), false)
            }
        } else {
            let (uint, overflow) = self.uint.overflowing_shr(rhs);
            (Self { uint }, overflow)
        }
    }
    #[inline]
    pub const fn overflowing_abs(self) -> (Self, bool) {
        if self.is_negative() {
            self.overflowing_neg()
        } else {
            (self, false)
        }
    }
    overflowing_pow!();
}

use core::ops::{Div, Rem};

impl<const N: usize> const Div for Bint<N> {
    type Output = Self;

    fn div(self, rhs: Self) -> Self {
        if self.eq(&Self::MIN) && rhs.eq(&Self::NEG_ONE) {
            panic!("attempt to divide with overflow")
        } else {
            if rhs.is_zero() {
                div_zero!()
            }
            self.div_rem_unchecked(rhs).0
        }
    }
}

op_ref_impl!(Div<Bint<N>> for Bint<N>, div);

impl<const N: usize> const Rem for Bint<N> {
    type Output = Self;

    fn rem(self, rhs: Self) -> Self {
        if self.eq(&Self::MIN) && rhs.eq(&Self::NEG_ONE) {
            panic!("attempt to calculate remainder with overflow")
        } else {
            if rhs.is_zero() {
                rem_zero!()
            }
            self.div_rem_unchecked(rhs).1
        }
    }
}

op_ref_impl!(Rem<Bint<N>> for Bint<N>, rem);

#[cfg(test)]
mod tests {
    use crate::{I128};

    fn converter((i, b): (i128, bool)) -> (I128, bool) {
        (i.into(), b)
    }

    test_signed! {
        function: overflowing_add(a: i128, b: i128),
        cases: [
            (-i128::MAX, i128::MIN),
            (i128::MAX, i128::MAX)
        ],
        converter: converter
    }
    test_signed! {
        function: overflowing_sub(a: i128, b: i128),
        cases: [
            (i128::MIN, 13i128),
            (i128::MAX, -1i128),
            (i128::MAX, i128::MIN)
        ],
        converter: converter
    }
    test_signed! {
        function: overflowing_mul(a: i128, b: i128),
        cases: [
            (1i128 << 64, 1i128 << 63),
            (-(1i128 << 100), 1i128 << 27)
        ],
        converter: converter
    }
    test_signed! {
        function: overflowing_div(a: i128, b: i128),
        cases: [
            (-1i128, 2i128),
            (i128::MIN, -1i128)
        ],
        quickcheck_skip: b == 0,
        converter: converter
    }
    test_signed! {
        function: overflowing_div_euclid(a: i128, b: i128),
        cases: [
            (-1i128, 2i128),
            (i128::MIN, -1i128)
        ],
        quickcheck_skip: b == 0,
        converter: converter
    }
    test_signed! {
        function: overflowing_rem(a: i128, b: i128),
        cases: [
            (-577i128 * 80456498576, 577i128),
            (i128::MIN, -1i128)
        ],
        quickcheck_skip: b == 0,
        converter: converter
    }
    test_signed! {
        function: overflowing_rem_euclid(a: i128, b: i128),
        cases: [
            (0i128, -79872976456456i128),
            (i128::MIN, -1i128),
            (-1i128, 2i128)
        ],
        quickcheck_skip: b == 0,
        converter: converter
    }
    test_signed! {
        function: overflowing_neg(a: i128),
        cases: [
            (0i128),
            (i128::MIN),
            (997340597745960395879i128)
        ],
        converter: converter
    }
    test_signed! {
        function: overflowing_shl(a: i128, b: u16),
        cases: [
            (i128::MAX - 3453475, 8 as u16),
            (77948798i128, 58743 as u16),
            (-9797456456456i128, 27 as u16)
        ],
        converter: converter
    }
    test_signed! {
        function: overflowing_shr(a: i128, b: u16),
        cases: [
            (i128::MIN, 11 as u16),
            (-1i128, 85 as u16)
        ],
        converter: converter
    }
    test_signed! {
        function: overflowing_pow(a: i128, b: u16),
        cases: [
            (97465984i128, 2555 as u16),
            (-19i128, 11 as u16),
            (-277i128, 14 as u16)
        ],
        converter: converter
    }
}