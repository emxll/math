use std::{
    cmp::{Ordering, PartialOrd, max},
    iter::repeat,
    ops::{Add, Sub},
};

pub trait PrimitiveInteger: Add<Self> + Copy + std::fmt::Debug + Default + Ord + 'static {
    fn maximum() -> Self;
    fn bit_capacity() -> usize {
        Self::maximum().bit_len()
    }
    fn bits_to_digits(bits: usize) -> usize {
        (bits + Self::bit_capacity() - 1) / Self::bit_capacity()
    }
    fn bit_len(&self) -> usize;
    fn add_with_carry(&self, carry: bool, other: &Self) -> (Self, bool);
    fn sub_with_borrow(&self, borrow: bool, other: &Self) -> (Self, bool);
    fn get_bit(&self, n: usize) -> bool;
}

#[derive(Debug)]
pub enum DigitBuf<'a, Digit: PrimitiveInteger> {
    Owned(Box<[Digit]>),
    Borrowed(&'a mut [Digit]),
    Single([Digit; 1]),
}

impl<Digit: PrimitiveInteger> DigitBuf<'_, Digit> {
    fn inner(&self) -> &[Digit] {
        match self {
            DigitBuf::Owned(items) => items,
            DigitBuf::Borrowed(items) => items,
            DigitBuf::Single(items) => items,
        }
    }
    fn inner_mut(&mut self) -> &mut [Digit] {
        match self {
            DigitBuf::Owned(items) => items,
            DigitBuf::Borrowed(items) => items,
            DigitBuf::Single(items) => items.as_mut_slice(),
        }
    }

    fn get_bit(&self, n: usize) -> bool {
        let digit = Digit::bits_to_digits(n + 1) - 1;
        self.inner()
            .get(digit)
            .map(|val| val.get_bit(n % Digit::bit_capacity()))
            .unwrap_or(false)
    }

    fn from_digit(digit: Digit) -> DigitBuf<'static, Digit> {
        DigitBuf::Single([digit; 1])
    }

    fn new(bits: usize) -> DigitBuf<'static, Digit> {
        let digits = Digit::bits_to_digits(bits);
        DigitBuf::new_digits(digits)
    }

    fn new_digits(digits: usize) -> DigitBuf<'static, Digit> {
        if digits == 1 {
            return DigitBuf::Single([Digit::default(); 1]);
        }
        DigitBuf::Owned(vec![Digit::default(); digits].into_boxed_slice())
    }

    fn abs_add_into<'a>(
        &self,
        other: &DigitBuf<'_, Digit>,
        buf: &mut DigitBuf<'a, Digit>,
        //the minimum required space of
        output_max_digits: usize,
    ) {
        let digits = buf.inner_mut();
        assert!(digits.len() >= output_max_digits);

        let left_iter = self
            .inner()
            .into_iter()
            .map(|e| *e)
            .chain(repeat(Digit::default()));
        let right_iter = other
            .inner()
            .into_iter()
            .map(|e| *e)
            .chain(repeat(Digit::default()));

        //shouldn't produce overflow
        assert!(
            !digits
                .iter_mut()
                .take(output_max_digits)
                .zip(left_iter.zip(right_iter))
                .fold(false, |carry, (slot, (left, right))| {
                    let (v, carry) = left.add_with_carry(carry, &right);
                    *slot = v;
                    carry
                })
        );
    }

    // Subtract a SMALLER number (other) from a LARGER number (self)
    // and store the result in buf
    //
    // RETURNS: The bit length of the resulting DigitBuf
    fn abs_sub_into<'a>(
        &self,
        other: &DigitBuf<'_, Digit>,
        buf: &mut DigitBuf<'a, Digit>,
        //the minimum required space of the subtraction
        output_max_digits: usize,
    ) -> usize {
        let digits = buf.inner_mut();
        assert!(digits.len() >= output_max_digits);

        let left_iter = self
            .inner()
            .into_iter()
            .map(|e| *e)
            .chain(repeat(Digit::default()));
        let right_iter = other
            .inner()
            .into_iter()
            .map(|e| *e)
            .chain(repeat(Digit::default()));

        //shouldn't produce overflow
        assert!(
            !digits
                .iter_mut()
                .take(output_max_digits)
                .zip(left_iter.zip(right_iter))
                .fold(false, |carry, (slot, (left, right))| {
                    let (v, carry) = left.sub_with_borrow(carry, &right);
                    *slot = v;
                    carry
                })
        );

        digits
            .iter()
            .take(output_max_digits)
            .enumerate()
            .rev()
            .filter_map(|(i, digit)| {
                (*digit != Digit::default()).then(|| i * Digit::bit_capacity() + digit.bit_len())
            })
            .next()
            .expect("DigitBuf subtraction doesn't produce 0")
    }
}

#[derive(Debug)]
pub struct GenericInteger<'buf, Digit: PrimitiveInteger> {
    //is only guaranteed to be sane up to the bit indicated by 'bits'
    digits: DigitBuf<'buf, Digit>,
    // 0 by convention iff the number is zero
    bits: usize,
    // may not be negative when zero
    negative: bool,
}

impl<Digit: PrimitiveInteger> GenericInteger<'_, Digit> {
    pub fn get_bit(&self, n: usize) -> bool {
        self.digits.get_bit(n)
    }

    // Zero is not negative
    pub fn negative(&self) -> bool {
        self.negative
    }

    pub fn abs_cmp(&self, other: &Self) -> Ordering {
        if self.bits != other.bits {
            return self.bits.cmp(&other.bits);
        }
        let cmp_digits = Digit::bits_to_digits(self.bits);
        self.digits
            .inner()
            .iter()
            .zip(other.digits.inner().iter())
            .take(cmp_digits)
            .rev()
            .map(|(my_digit, other_digit)| my_digit.cmp(other_digit))
            .filter(|ord| *ord != Ordering::Equal)
            .next()
            .unwrap_or(Ordering::Equal)
    }

    fn cmp(&self, other: &GenericInteger<'_, Digit>) -> std::cmp::Ordering {
        match (self.negative(), other.negative()) {
            (false, false) => self.abs_cmp(other),
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            (true, true) => other.abs_cmp(self),
        }
    }

    fn additive_size_bound(&self, other: &GenericInteger<'_, Digit>) -> usize {
        max(self.bits, other.bits) + 1
    }

    fn subtractive_size_bound(&self, other: &GenericInteger<'_, Digit>) -> usize {
        max(self.bits, other.bits)
    }

    fn addsub_multiplex_into<'a>(
        &self,
        other: &GenericInteger<'_, Digit>,
        flip_other: bool,
        mut buf: DigitBuf<'a, Digit>,
    ) -> GenericInteger<'a, Digit> {
        let subtract = self.negative ^ other.negative() ^ flip_other;
        let mut result_negative = self.negative();
        if subtract {
            let (big, small) = match self.abs_cmp(other) {
                Ordering::Equal => {
                    return GenericInteger::from(Digit::default());
                }
                Ordering::Less => {
                    result_negative = !result_negative;
                    (other, self)
                }
                Ordering::Greater => (self, other),
            };
            let output_digits = Digit::bits_to_digits(self.subtractive_size_bound(other));
            let bits = big
                .digits
                .abs_sub_into(&small.digits, &mut buf, output_digits);
            GenericInteger {
                digits: buf,
                bits,
                negative: result_negative,
            }
        } else {
            let output_bits = self.additive_size_bound(other);
            let output_digits = Digit::bits_to_digits(output_bits);

            self.digits
                .abs_add_into(&other.digits, &mut buf, output_digits);

            GenericInteger {
                bits: if buf.get_bit(output_bits - 1) {
                    output_bits
                } else {
                    output_bits - 1
                },
                digits: buf,
                negative: false,
            }
        }
    }

    fn add_into<'a>(
        &self,
        other: &GenericInteger<'_, Digit>,
        buf: DigitBuf<'a, Digit>,
    ) -> GenericInteger<'a, Digit> {
        self.addsub_multiplex_into(other, false, buf)
    }

    fn sub_into<'a>(
        &self,
        other: &GenericInteger<'_, Digit>,
        buf: DigitBuf<'a, Digit>,
    ) -> GenericInteger<'a, Digit> {
        self.addsub_multiplex_into(other, true, buf)
    }
}

impl<Digit: PrimitiveInteger> From<Digit> for GenericInteger<'_, Digit> {
    fn from(value: Digit) -> GenericInteger<'static, Digit> {
        GenericInteger {
            digits: DigitBuf::from_digit(value),
            bits: value.bit_len(),
            negative: false,
        }
    }
}

impl<Digit: PrimitiveInteger> Ord for GenericInteger<'_, Digit> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.cmp(other)
    }
}

impl<Digit: PrimitiveInteger> PartialOrd for GenericInteger<'_, Digit> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<Digit: PrimitiveInteger> PartialEq for GenericInteger<'_, Digit> {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl<Digit: PrimitiveInteger> Eq for GenericInteger<'_, Digit> {}

impl<Digit: PrimitiveInteger> Add<GenericInteger<'_, Digit>> for GenericInteger<'_, Digit> {
    type Output = GenericInteger<'static, Digit>;

    fn add(self, rhs: GenericInteger<'_, Digit>) -> Self::Output {
        let buf = DigitBuf::new(self.additive_size_bound(&rhs));
        self.add_into(&rhs, buf)
    }
}

impl<Digit: PrimitiveInteger> Sub<GenericInteger<'_, Digit>> for GenericInteger<'_, Digit> {
    type Output = GenericInteger<'static, Digit>;

    fn sub(self, rhs: GenericInteger<'_, Digit>) -> Self::Output {
        let buf = DigitBuf::new(self.subtractive_size_bound(&rhs));
        self.sub_into(&rhs, buf)
    }
}
impl PrimitiveInteger for u64 {
    fn maximum() -> Self {
        u64::MAX
    }

    fn bit_len(&self) -> usize {
        (1..64).filter(|i| self >> i == 0).next().unwrap_or(64)
    }

    fn add_with_carry(&self, carry: bool, other: &Self) -> (Self, bool) {
        // Workaround until stabilization:
        // https://doc.rust-lang.org/nightly/std/primitive.u64.html#method.carrying_add
        let (res1, carry1) = self.overflowing_add(carry as u64);
        let (res2, carry2) = res1.overflowing_add(*other);
        (res2, carry1 || carry2)
    }

    fn sub_with_borrow(&self, borrow: bool, other: &Self) -> (Self, bool) {
        // Workaround until stabilization:
        // https://doc.rust-lang.org/nightly/std/primitive.u64.html#method.borrowing_sub
        let (res1, borrow1) = self.overflowing_sub(borrow as u64);
        let (res2, borrow2) = res1.overflowing_sub(*other);
        (res2, borrow1 || borrow2)
    }

    fn get_bit(&self, n: usize) -> bool {
        (self >> n) & 1 != 0
    }
}

pub type Integer = GenericInteger<'static, u64>;
