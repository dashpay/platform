use crate::Value;

macro_rules! implpartialeq {
    ($($t:ty),+ $(,)?) => {
        $(
            impl PartialEq<$t> for Value {
                #[inline]
                fn eq(&self, other: &$t) -> bool {
                    if let Some(i) = self.as_integer::<$t>() {
                        &i == other
                    } else {
                        false
                    }
                }
            }

            impl PartialEq<$t> for &Value {
                #[inline]
                fn eq(&self, other: &$t) -> bool {
                    if let Some(i) = self.as_integer::<$t>() {
                        &i == other
                    } else {
                        false
                    }
                }
            }
        )+
    };
}

implpartialeq! {
    u128,
    u64,
    u32,
    u16,
    u8,
    i128,
    i64,
    i32,
    i16,
    i8,
}

impl PartialEq<String> for Value {
    #[inline]
    fn eq(&self, other: &String) -> bool {
        if let Some(i) = self.as_text() {
            i == other
        } else {
            false
        }
    }
}

impl PartialEq<String> for &Value {
    #[inline]
    fn eq(&self, other: &String) -> bool {
        if let Some(i) = self.as_str() {
            i == other
        } else {
            false
        }
    }
}

impl PartialEq<&str> for Value {
    #[inline]
    fn eq(&self, other: &&str) -> bool {
        if let Some(i) = self.as_str() {
            &i == other
        } else {
            false
        }
    }
}

impl PartialEq<&str> for &Value {
    #[inline]
    fn eq(&self, other: &&str) -> bool {
        if let Some(i) = self.as_str() {
            &i == other
        } else {
            false
        }
    }
}

impl PartialEq<f64> for Value {
    #[inline]
    fn eq(&self, other: &f64) -> bool {
        if let Some(i) = self.as_float() {
            &i == other
        } else {
            false
        }
    }
}

impl PartialEq<f64> for &Value {
    #[inline]
    fn eq(&self, other: &f64) -> bool {
        if let Some(i) = self.as_float() {
            &i == other
        } else {
            false
        }
    }
}

impl PartialEq<Vec<u8>> for Value {
    #[inline]
    fn eq(&self, other: &Vec<u8>) -> bool {
        self.as_bytes_slice() == Ok(other.as_slice())
    }
}
impl PartialEq<Vec<u8>> for &Value {
    #[inline]
    fn eq(&self, other: &Vec<u8>) -> bool {
        self.as_bytes_slice() == Ok(other.as_slice())
    }
}

macro_rules! impl_bytes_array_eq {
    ($($n:expr),+ $(,)?) => {$(
        impl PartialEq<[u8; $n]> for Value {
            #[inline]
            fn eq(&self, other: &[u8; $n]) -> bool {
                self.as_bytes_slice() == Ok(other.as_slice())
            }
        }
        impl PartialEq<[u8; $n]> for &Value {
            #[inline]
            fn eq(&self, other: &[u8; $n]) -> bool {
                self.as_bytes_slice() == Ok(other.as_slice())
            }
        }
    )+};
}
impl_bytes_array_eq! { 20, 32, 36 }

impl Value {
    /* -------------------------------------------------------- *
     *  equality on underlying data                             *
     * -------------------------------------------------------- */

    /// Returns `true` when the *data* represented by the two `Value`s
    /// is identical, even if they are stored in different but
    /// compatible variants.
    ///
    /// * All “bytes-like” variants (`Bytes`, `Bytes20`, `Bytes32`,
    ///   `Bytes36`, `Identifier`) compare equal when their byte
    ///   sequences match.
    /// * All integer variants (`U*`, `I*`) compare equal when they
    ///   represent the same numeric value.
    /// * Two `Map`s compare equal when they hold the same keys, in any
    ///   order, each with equal underlying data; two `Array`s when they
    ///   have the same length and equal underlying data position by
    ///   position. Both recurse with the same rules, so a nested integer
    ///   stored at a narrower width, or an object whose members were
    ///   reordered by schema position, still compares equal.
    /// * Otherwise falls back to normal `==` (`PartialEq`) behaviour.
    ///
    /// Shipped generations call this at every protocol version: the document
    /// replace transition transformer v0 uses it to decide which fields a
    /// replace changed, and `is_equal_ignoring_timestamps` v0 uses it when a
    /// client verifies a state transition proof. A change to its result for
    /// some pair of values changes their output everywhere; make such a change
    /// a new versioned method instead of editing this function.
    pub fn equal_underlying_data(&self, other: &Value) -> bool {
        // 1) bytes-like cross-variant equality
        if let (Ok(a), Ok(b)) = (self.as_bytes_slice(), other.as_bytes_slice()) {
            return a == b;
        }

        // 2) integer cross-variant equality
        if let (Some(a), Some(b)) = (self.as_i128_unified(), other.as_i128_unified()) {
            return a == b;
        }

        // 3) containers, recursively and (for maps) regardless of member order
        match (self, other) {
            (Value::Map(this), Value::Map(that)) => {
                if this.len() != that.len() {
                    return false;
                }
                // One-to-one: a map is a list of pairs, so a duplicated key
                // on one side must not be satisfied twice by a single entry
                // on the other. Each matched entry is consumed.
                let mut consumed = vec![false; that.len()];
                for (key, value) in this {
                    let matched =
                        that.iter()
                            .enumerate()
                            .find(|(index, (other_key, other_value))| {
                                !consumed[*index]
                                    && key.equal_underlying_data(other_key)
                                    && value.equal_underlying_data(other_value)
                            });
                    match matched {
                        Some((index, _)) => consumed[index] = true,
                        None => return false,
                    }
                }
                true
            }
            (Value::Array(this), Value::Array(that)) => {
                this.len() == that.len()
                    && this
                        .iter()
                        .zip(that)
                        .all(|(value, other_value)| value.equal_underlying_data(other_value))
            }
            // 4) default
            _ => self == other,
        }
    }

    /// Returns `true` when two single values hold the same data:
    /// [`equal_underlying_data`](Self::equal_underlying_data) for values that
    /// are not containers, with floats compared by their bits.
    ///
    /// * All "bytes-like" variants (`Bytes`, `Bytes20`, `Bytes32`, `Bytes36`,
    ///   `Identifier`) compare equal when their byte sequences match, so an
    ///   identifier equals the same 32 bytes carried as `Bytes32` or `Bytes`.
    ///   `Text` is not bytes-like: a string never equals bytes.
    /// * All integer variants (`U*`, `I*`) compare equal when they represent
    ///   the same numeric value, whatever width each is carried at. A `U128`
    ///   past `i128::MAX` equals only the same `U128`.
    /// * When either side is a `Float`, both sides are read as an `f64` and
    ///   compared by bits (`f64::to_bits`), not with `==`: `-0.0` is not
    ///   `0.0`, as their document tree keys are not, and a NaN equals a NaN
    ///   of the same bits, itself included. This is bit equality, not tree
    ///   key equality: the tree key encoding maps a few distinct bit patterns
    ///   (a negative NaN and a negative subnormal) to one key.
    ///   An integer on the other side is read as the `f64` it converts to, as
    ///   [`as_float`](Self::as_float) reads it: a document stores `date` and
    ///   `number` properties as floats while a transition may carry one as an
    ///   integer. Past 2^53 an integer rounds to the nearest `f64`, as it does
    ///   when stored, so the comparison is not transitive there: `2^53 + 1`
    ///   and `2^53` both equal the float `2^53` but not each other.
    /// * An `Array` whose every element is a `U8` holds the bytes it lists, as
    ///   [`to_identifier_bytes`](Self::to_identifier_bytes) reads it, so it
    ///   equals the same bytes carried as bytes, an identifier, or another
    ///   such array: a transition may carry an identifier or a byte array
    ///   that way. Any other `Array`, and a `Map` on either side, is not a
    ///   single value and never compares equal, not even to an equal
    ///   container.
    /// * Otherwise the two must be the same variant and compare with `==`:
    ///   two `Null`s are equal and a `Null` equals nothing else, text compares
    ///   as text (so `""` is not `"\0"`, though a tree key encodes both as
    ///   `[0]`), and booleans as booleans.
    ///
    /// A value of any size compares: two equal strings or byte arrays longer
    /// than the 255 bytes a tree key holds are equal.
    ///
    /// From protocol version 14, document reference validation 0 judges each
    /// `propertyAgreement` pair of a `refersTo` reference with it. A change to
    /// its result for some pair of values changes which documents are
    /// accepted; make such a change a new method instead of editing this one.
    pub fn same_scalar_data(&self, other: &Value) -> bool {
        /// The bytes `value` holds: a bytes-like variant's, or those an array
        /// of `U8`s lists. `None` for anything else.
        fn held_bytes(value: &Value) -> Option<Vec<u8>> {
            match value {
                Value::Array(items) => items
                    .iter()
                    .map(|item| match item {
                        Value::U8(byte) => Some(*byte),
                        _ => None,
                    })
                    .collect(),
                _ => value.as_bytes_slice().ok().map(<[u8]>::to_vec),
            }
        }

        match (self, other) {
            // 1) an array is a single value only as the bytes it lists
            (Value::Array(_), _) | (_, Value::Array(_)) => matches!(
                (held_bytes(self), held_bytes(other)),
                (Some(this), Some(that)) if this == that
            ),
            // 2) a map is no single value
            (Value::Map(_), _) | (_, Value::Map(_)) => false,
            // 3) floats by bits, an integer read as the float it converts to
            (Value::Float(_), _) | (_, Value::Float(_)) => matches!(
                (self.as_float(), other.as_float()),
                (Some(this), Some(that)) if this.to_bits() == that.to_bits()
            ),
            // 4) bytes by bytes, integers by value, anything else by `==`
            _ => self.equal_underlying_data(other),
        }
    }
}

#[cfg(test)]
mod underlying_data_tests {
    use crate::Value;

    fn map(entries: Vec<(&str, Value)>) -> Value {
        Value::Map(
            entries
                .into_iter()
                .map(|(key, value)| (Value::Text(key.to_string()), value))
                .collect(),
        )
    }

    /// The two ways storage rewrites an object a client sent: integer
    /// widths shrink to the smallest fitting variant and members are
    /// reordered by schema position.
    #[test]
    fn maps_compare_equal_across_member_order_and_integer_width() {
        let sent = map(vec![
            ("rank", Value::U64(7)),
            ("tag", Value::Text("intro".into())),
        ]);
        let stored = map(vec![
            ("tag", Value::Text("intro".into())),
            ("rank", Value::U8(7)),
        ]);

        assert!(sent.equal_underlying_data(&stored));
        assert!(stored.equal_underlying_data(&sent));
    }

    #[test]
    fn maps_differ_on_a_changed_nested_value() {
        let before = map(vec![
            ("rank", Value::U64(7)),
            ("tag", Value::Text("intro".into())),
        ]);
        let after = map(vec![
            ("tag", Value::Text("intro".into())),
            ("rank", Value::U8(8)),
        ]);

        assert!(!before.equal_underlying_data(&after));
    }

    #[test]
    fn maps_differ_on_a_missing_or_extra_member() {
        let one = map(vec![("tag", Value::Text("intro".into()))]);
        let two = map(vec![
            ("tag", Value::Text("intro".into())),
            ("rank", Value::U8(7)),
        ]);

        assert!(!one.equal_underlying_data(&two));
        assert!(!two.equal_underlying_data(&one));
    }

    #[test]
    fn nested_maps_recurse() {
        let sent = map(vec![(
            "meta",
            map(vec![
                ("rank", Value::I64(7)),
                ("tag", Value::Text("a".into())),
            ]),
        )]);
        let stored = map(vec![(
            "meta",
            map(vec![
                ("tag", Value::Text("a".into())),
                ("rank", Value::U8(7)),
            ]),
        )]);

        assert!(sent.equal_underlying_data(&stored));
    }

    #[test]
    fn arrays_compare_position_by_position_with_integer_leniency() {
        let sent = Value::Array(vec![Value::U64(1), Value::U64(2)]);
        let stored = Value::Array(vec![Value::U8(1), Value::U8(2)]);
        let reordered = Value::Array(vec![Value::U8(2), Value::U8(1)]);
        let shorter = Value::Array(vec![Value::U8(1)]);

        assert!(sent.equal_underlying_data(&stored));
        assert!(!sent.equal_underlying_data(&reordered));
        assert!(!sent.equal_underlying_data(&shorter));
    }

    /// A map is a list of pairs, so a key can in principle appear twice.
    /// Two entries on one side must not both be satisfied by the single
    /// entry they match on the other.
    #[test]
    fn maps_match_entries_one_to_one() {
        let doubled = map(vec![("tag", Value::U8(1)), ("tag", Value::U8(1))]);
        let single_plus_other = map(vec![("tag", Value::U8(1)), ("rank", Value::U8(1))]);
        let doubled_too = map(vec![("tag", Value::U64(1)), ("tag", Value::U64(1))]);

        assert!(!doubled.equal_underlying_data(&single_plus_other));
        assert!(!single_plus_other.equal_underlying_data(&doubled));
        assert!(doubled.equal_underlying_data(&doubled_too));
    }

    #[test]
    fn a_map_never_equals_a_non_map() {
        let a_map = map(vec![("tag", Value::Text("a".into()))]);

        assert!(!a_map.equal_underlying_data(&Value::Text("a".into())));
        assert!(!a_map.equal_underlying_data(&Value::Array(vec![])));
    }
}

#[cfg(test)]
mod same_scalar_data_tests {
    use crate::Value;

    /// Compares both ways, checking the two directions agree.
    fn same(left: &Value, right: &Value) -> bool {
        let forward = left.same_scalar_data(right);
        assert_eq!(
            forward,
            right.same_scalar_data(left),
            "{left:?} and {right:?} compare differently each way"
        );
        forward
    }

    /// A tree key encodes both as `[0]`; as values they differ.
    #[test]
    fn should_not_equate_the_empty_string_with_a_nul_character() {
        let empty = Value::Text(String::new());
        let nul = Value::Text("\0".to_string());

        assert!(!same(&empty, &nul));
        assert!(same(&empty, &empty.clone()));
        assert!(same(&nul, &nul.clone()));
    }

    /// A transition may carry an integer at another width than the stored
    /// document decodes it at.
    #[test]
    fn should_equate_the_same_integer_carried_at_different_widths() {
        assert!(same(&Value::U64(513), &Value::U16(513)));
        assert!(same(&Value::I64(-7), &Value::I32(-7)));
        assert!(same(&Value::U8(100), &Value::I128(100)));
        assert!(!same(&Value::U64(514), &Value::U16(513)));
        assert!(!same(&Value::I8(-1), &Value::U64(255)));
    }

    #[test]
    fn should_equate_a_u128_past_i128_only_with_the_same_u128() {
        let past_i128 = u128::MAX;

        assert!(same(&Value::U128(past_i128), &Value::U128(past_i128)));
        assert!(!same(&Value::U128(past_i128), &Value::U128(past_i128 - 1)));
        assert!(!same(
            &Value::U128(i128::MAX as u128 + 1),
            &Value::I128(i128::MAX)
        ));
        assert!(!same(&Value::U128(past_i128), &Value::U64(u64::MAX)));
        assert!(same(
            &Value::U128(i128::MAX as u128),
            &Value::I128(i128::MAX)
        ));
    }

    #[test]
    fn should_equate_an_identifier_with_the_same_32_bytes() {
        let id = [7u8; 32];

        assert!(same(&Value::Identifier(id), &Value::Bytes32(id)));
        assert!(same(&Value::Bytes(id.to_vec()), &Value::Identifier(id)));
        assert!(!same(&Value::Identifier(id), &Value::Identifier([8u8; 32])));
        assert!(!same(&Value::Identifier(id), &Value::Bytes(vec![7u8; 31])));
    }

    /// No tree key holds more than 255 bytes; a value of any size compares.
    #[test]
    fn should_equate_equal_strings_and_byte_arrays_longer_than_a_tree_key() {
        let long_ascii = Value::Text("a".repeat(256));
        // 280 bytes of UTF-8 in 70 characters
        let long_emoji = Value::Text("\u{1F600}".repeat(70));

        assert!(same(&long_ascii, &long_ascii.clone()));
        assert!(same(&long_emoji, &long_emoji.clone()));
        assert!(!same(&long_ascii, &Value::Text("a".repeat(257))));
        assert!(same(
            &Value::Bytes(vec![9; 300]),
            &Value::Bytes(vec![9; 300])
        ));
        assert!(!same(
            &Value::Bytes(vec![9; 300]),
            &Value::Bytes(vec![9; 301])
        ));
    }

    /// Floats compare by bits, where `equal_underlying_data` compares them
    /// with `==`.
    #[test]
    fn should_compare_floats_by_their_bits() {
        let quiet_nan = Value::Float(f64::NAN);
        let other_nan = Value::Float(f64::from_bits(f64::NAN.to_bits() | 1));

        assert!(!same(&Value::Float(-0.0), &Value::Float(0.0)));
        assert!(Value::Float(-0.0).equal_underlying_data(&Value::Float(0.0)));
        assert!(same(&Value::Float(1.5), &Value::Float(1.5)));
        assert!(!same(&Value::Float(1.5), &Value::Float(2.5)));
        assert!(same(&quiet_nan, &quiet_nan.clone()));
        assert!(!quiet_nan.equal_underlying_data(&quiet_nan.clone()));
        assert!(!same(&quiet_nan, &other_nan));
    }

    /// A document stores a date as a float; a transition may carry it as an
    /// integer.
    #[test]
    fn should_read_an_integer_against_a_float_as_the_float_it_converts_to() {
        assert!(same(&Value::U64(1_700_000_000_000), &Value::Float(1.7e12)));
        assert!(!same(&Value::U64(1_700_000_000_001), &Value::Float(1.7e12)));
        assert!(same(&Value::I32(-3), &Value::Float(-3.0)));
        assert!(same(&Value::U64(0), &Value::Float(0.0)));
        assert!(!same(&Value::U64(0), &Value::Float(-0.0)));
        assert!(!same(&Value::Text("1".to_string()), &Value::Float(1.0)));
        assert!(!same(&Value::Bool(true), &Value::Float(1.0)));
    }

    /// Past 2^53 an integer rounds to the nearest `f64`, as storage rounds
    /// it, so two integers that differ can both equal one float.
    #[test]
    fn should_round_an_integer_past_2_pow_53_to_the_nearest_float() {
        let two_pow_53 = 1u64 << 53;
        let float_two_pow_53 = Value::Float(two_pow_53 as f64);

        assert!(same(&Value::U64(two_pow_53 + 1), &float_two_pow_53));
        assert!(same(&Value::U64(two_pow_53), &float_two_pow_53));
        assert!(!same(&Value::U64(two_pow_53 + 1), &Value::U64(two_pow_53)));

        assert!(same(
            &Value::I128(i128::MIN),
            &Value::Float(-(2.0f64.powi(127)))
        ));
        assert!(same(
            &Value::U128(u128::MAX),
            &Value::Float(2.0f64.powi(128))
        ));
        assert!(!same(&Value::U128(u128::MAX), &Value::Float(f64::INFINITY)));
    }

    #[test]
    fn should_never_equate_a_map_or_an_array_of_anything_but_bytes() {
        let empty_map = Value::Map(vec![]);
        let one_entry_map = Value::Map(vec![(Value::Text("a".into()), Value::U8(1))]);
        let wide_integers = Value::Array(vec![Value::U64(1), Value::U64(2)]);
        let texts = Value::Array(vec![Value::Text("a".into())]);

        assert!(!same(&empty_map, &empty_map.clone()));
        assert!(!same(&one_entry_map, &one_entry_map.clone()));
        assert!(one_entry_map.equal_underlying_data(&one_entry_map.clone()));
        assert!(!same(&wide_integers, &wide_integers.clone()));
        assert!(!same(&wide_integers, &Value::Bytes(vec![1, 2])));
        assert!(!same(&texts, &texts.clone()));
        assert!(!same(&empty_map, &Value::Null));
        assert!(!same(&empty_map, &Value::Array(vec![])));
    }

    /// A transition may carry an identifier or a byte array as an array of
    /// `U8`s, which `to_identifier_bytes` reads as those bytes.
    #[test]
    fn should_read_an_array_of_u8_as_the_bytes_it_lists() {
        let id = [7u8; 32];
        let listed = Value::Array(id.iter().copied().map(Value::U8).collect());

        assert!(same(&listed, &Value::Identifier(id)));
        assert!(same(&listed, &Value::Bytes32(id)));
        assert!(same(&listed, &Value::Bytes(id.to_vec())));
        assert!(same(&listed, &listed.clone()));
        assert!(same(&Value::Array(vec![]), &Value::Bytes(vec![])));
        assert!(!same(&listed, &Value::Identifier([8u8; 32])));
        assert!(!same(&Value::Array(vec![Value::U8(1)]), &Value::Float(1.0)));
        assert!(!same(&Value::Array(vec![Value::U8(1)]), &Value::U8(1)));
        assert!(!same(
            &Value::Array(vec![Value::U8(97)]),
            &Value::Text("a".into())
        ));
        assert!(!same(&Value::Array(vec![Value::U8(1)]), &Value::Null));
    }

    #[test]
    fn should_equate_two_nulls_and_nothing_else_with_a_null() {
        assert!(same(&Value::Null, &Value::Null));
        assert!(!same(&Value::Null, &Value::Text(String::new())));
        assert!(!same(&Value::Null, &Value::Bytes(vec![])));
        assert!(!same(&Value::Null, &Value::U64(0)));
        assert!(!same(&Value::Null, &Value::Bool(false)));
        assert!(!same(&Value::Null, &Value::Float(0.0)));
    }

    #[test]
    fn should_not_equate_values_of_different_kinds() {
        assert!(!same(&Value::Text("1".to_string()), &Value::U64(1)));
        assert!(!same(&Value::Bool(true), &Value::U8(1)));
        assert!(!same(&Value::U64(1), &Value::Bool(true)));
        assert!(!same(
            &Value::Text("abc".to_string()),
            &Value::Bytes(b"abc".to_vec())
        ));
        assert!(same(&Value::Bool(true), &Value::Bool(true)));
        assert!(!same(&Value::Bool(true), &Value::Bool(false)));
    }
}

#[cfg(test)]
#[allow(clippy::approx_constant)]
mod tests {
    use crate::Value;

    // ---- PartialEq<integer types> ----

    #[test]
    fn u8_eq() {
        assert_eq!(Value::U8(42), 42u8);
        assert_ne!(Value::U8(42), 43u8);
    }

    #[test]
    fn i8_eq() {
        assert_eq!(Value::I8(-1), -1i8);
        assert_ne!(Value::I8(-1), 0i8);
    }

    #[test]
    fn u16_eq() {
        assert_eq!(Value::U16(1000), 1000u16);
        assert_ne!(Value::U16(1000), 999u16);
    }

    #[test]
    fn i16_eq() {
        assert_eq!(Value::I16(-500), -500i16);
        assert_ne!(Value::I16(-500), 500i16);
    }

    #[test]
    fn u32_eq() {
        assert_eq!(Value::U32(100_000), 100_000u32);
        assert_ne!(Value::U32(100_000), 0u32);
    }

    #[test]
    fn i32_eq() {
        assert_eq!(Value::I32(-100), -100i32);
        assert_ne!(Value::I32(-100), 100i32);
    }

    #[test]
    fn u64_eq() {
        assert_eq!(Value::U64(u64::MAX), u64::MAX);
        assert_ne!(Value::U64(0), 1u64);
    }

    #[test]
    fn i64_eq() {
        assert_eq!(Value::I64(i64::MIN), i64::MIN);
        assert_ne!(Value::I64(0), 1i64);
    }

    #[test]
    fn u128_eq() {
        assert_eq!(Value::U128(u128::MAX), u128::MAX);
        assert_ne!(Value::U128(0), 1u128);
    }

    #[test]
    fn i128_eq() {
        assert_eq!(Value::I128(i128::MIN), i128::MIN);
        assert_ne!(Value::I128(0), 1i128);
    }

    // ---- cross-type integer comparison via as_integer ----

    #[test]
    fn u8_value_eq_u64_type() {
        // Value::U8(10) should equal 10u64 through as_integer
        assert_eq!(Value::U8(10), 10u64);
    }

    #[test]
    fn u64_value_eq_u8_type_when_fits() {
        assert_eq!(Value::U64(200), 200u8);
    }

    #[test]
    fn u64_value_ne_u8_type_when_overflow() {
        // 256 doesn't fit in u8
        assert_ne!(Value::U64(256), 0u8); // as_integer::<u8> returns None
    }

    #[test]
    fn i8_value_eq_i64_type() {
        assert_eq!(Value::I8(-10), -10i64);
    }

    #[test]
    fn non_integer_ne_integer() {
        assert_ne!(Value::Text("hello".to_string()), 0u64);
        assert_ne!(Value::Null, 0i32);
        assert_ne!(Value::Bool(true), 1u8);
    }

    // ---- PartialEq<String> ----

    #[test]
    fn string_eq() {
        let val = Value::Text("hello".to_string());
        assert_eq!(val, "hello".to_string());
        assert_ne!(val, "world".to_string());
    }

    #[test]
    fn non_text_ne_string() {
        assert_ne!(Value::U8(0), "0".to_string());
        assert_ne!(Value::Null, "".to_string());
    }

    // ---- PartialEq<&str> ----

    #[test]
    fn str_ref_eq() {
        let val = Value::Text("test".to_string());
        assert_eq!(val, "test");
        assert_ne!(val, "other");
    }

    #[test]
    fn non_text_ne_str_ref() {
        assert_ne!(Value::Bool(false), "false");
    }

    // ---- PartialEq<f64> ----

    #[test]
    fn float_eq() {
        assert_eq!(Value::Float(3.14), 3.14f64);
        assert_ne!(Value::Float(3.14), 3.15f64);
    }

    #[test]
    fn integer_eq_float_through_as_float() {
        // as_float converts integers to f64, so Value::U64(10) == 10.0f64
        assert_eq!(Value::U64(10), 10.0f64);
    }

    #[test]
    fn non_numeric_ne_float() {
        assert_ne!(Value::Text("3.14".to_string()), 3.14f64);
    }

    // ---- PartialEq<Vec<u8>> ----

    #[test]
    fn bytes_eq_vec_u8() {
        let data = vec![1, 2, 3];
        assert_eq!(Value::Bytes(data.clone()), data);
    }

    #[test]
    fn bytes_ne_vec_u8() {
        assert_ne!(Value::Bytes(vec![1, 2, 3]), vec![1, 2, 4]);
    }

    #[test]
    fn identifier_eq_vec_u8() {
        let id = [42u8; 32];
        assert_eq!(Value::Identifier(id), id.to_vec());
    }

    #[test]
    fn bytes20_eq_vec_u8() {
        let b = [5u8; 20];
        assert_eq!(Value::Bytes20(b), b.to_vec());
    }

    #[test]
    fn non_bytes_ne_vec_u8() {
        assert_ne!(Value::U8(1), vec![1u8]);
    }

    // ---- PartialEq<[u8; 32]> ----

    #[test]
    fn bytes32_eq_array() {
        let b = [0xffu8; 32];
        assert_eq!(Value::Bytes32(b), b);
    }

    #[test]
    fn identifier_eq_array_32() {
        let id = [7u8; 32];
        assert_eq!(Value::Identifier(id), id);
    }

    #[test]
    fn bytes_eq_array_32() {
        let data = [3u8; 32];
        assert_eq!(Value::Bytes(data.to_vec()), data);
    }

    #[test]
    fn non_bytes_ne_array_32() {
        assert_ne!(Value::Null, [0u8; 32]);
    }

    // ---- PartialEq<[u8; 20]> ----

    #[test]
    fn bytes20_eq_array_20() {
        let b = [1u8; 20];
        assert_eq!(Value::Bytes20(b), b);
    }

    // ---- PartialEq<[u8; 36]> ----

    #[test]
    fn bytes36_eq_array_36() {
        let b = [2u8; 36];
        assert_eq!(Value::Bytes36(b), b);
    }

    // ---- PartialEq for &Value ----

    #[test]
    fn ref_value_eq_integer() {
        let val = Value::U64(42);
        assert_eq!(&val, 42u64);
    }

    #[test]
    fn ref_value_eq_string() {
        let val = Value::Text("hi".to_string());
        assert_eq!(&val, "hi".to_string());
    }

    #[test]
    fn ref_value_eq_str_ref() {
        let val = Value::Text("hi".to_string());
        assert_eq!(&val, "hi");
    }

    #[test]
    fn ref_value_eq_float() {
        let val = Value::Float(1.0);
        assert_eq!(&val, 1.0f64);
    }

    #[test]
    fn ref_value_eq_vec_u8() {
        let val = Value::Bytes(vec![10, 20]);
        assert_eq!(&val, vec![10u8, 20]);
    }

    #[test]
    fn ref_value_eq_array_32() {
        let b = [0u8; 32];
        let val = Value::Bytes32(b);
        assert_eq!(&val, b);
    }

    // ---- equal_underlying_data tests ----

    #[test]
    fn equal_underlying_data_bytes_vs_identifier_same_data() {
        let data = [42u8; 32];
        let bytes = Value::Bytes(data.to_vec());
        let ident = Value::Identifier(data);
        assert!(bytes.equal_underlying_data(&ident));
        assert!(ident.equal_underlying_data(&bytes));
    }

    #[test]
    fn equal_underlying_data_bytes_vs_identifier_different_data() {
        let bytes = Value::Bytes(vec![0u8; 32]);
        let ident = Value::Identifier([1u8; 32]);
        assert!(!bytes.equal_underlying_data(&ident));
    }

    #[test]
    fn equal_underlying_data_bytes32_vs_identifier() {
        let data = [99u8; 32];
        let b32 = Value::Bytes32(data);
        let ident = Value::Identifier(data);
        assert!(b32.equal_underlying_data(&ident));
    }

    #[test]
    fn equal_underlying_data_bytes20_vs_bytes() {
        let data = [5u8; 20];
        let b20 = Value::Bytes20(data);
        let bytes = Value::Bytes(data.to_vec());
        assert!(b20.equal_underlying_data(&bytes));
    }

    #[test]
    fn equal_underlying_data_u8_vs_u64_same_value() {
        let a = Value::U8(10);
        let b = Value::U64(10);
        assert!(a.equal_underlying_data(&b));
    }

    #[test]
    fn equal_underlying_data_i8_vs_i128_same_value() {
        let a = Value::I8(-5);
        let b = Value::I128(-5);
        assert!(a.equal_underlying_data(&b));
    }

    #[test]
    fn equal_underlying_data_u8_vs_u64_different_value() {
        let a = Value::U8(10);
        let b = Value::U64(20);
        assert!(!a.equal_underlying_data(&b));
    }

    #[test]
    fn equal_underlying_data_u16_vs_i32_same_value() {
        let a = Value::U16(100);
        let b = Value::I32(100);
        assert!(a.equal_underlying_data(&b));
    }

    #[test]
    fn equal_underlying_data_negative_i8_vs_u64() {
        // negative can't match unsigned
        let a = Value::I8(-1);
        let b = Value::U64(255);
        assert!(!a.equal_underlying_data(&b));
    }

    #[test]
    fn equal_underlying_data_same_variant_same_value() {
        let a = Value::U64(42);
        let b = Value::U64(42);
        assert!(a.equal_underlying_data(&b));
    }

    #[test]
    fn equal_underlying_data_fallback_to_partial_eq() {
        // Text vs Text uses default PartialEq
        let a = Value::Text("hello".to_string());
        let b = Value::Text("hello".to_string());
        assert!(a.equal_underlying_data(&b));

        let c = Value::Text("world".to_string());
        assert!(!a.equal_underlying_data(&c));
    }

    #[test]
    fn equal_underlying_data_null_vs_null() {
        assert!(Value::Null.equal_underlying_data(&Value::Null));
    }

    #[test]
    fn equal_underlying_data_different_types_not_equal() {
        // A string vs a number should not be equal
        let a = Value::Text("42".to_string());
        let b = Value::U64(42);
        assert!(!a.equal_underlying_data(&b));
    }

    #[test]
    fn equal_underlying_data_bool_vs_bool() {
        assert!(Value::Bool(true).equal_underlying_data(&Value::Bool(true)));
        assert!(!Value::Bool(true).equal_underlying_data(&Value::Bool(false)));
    }

    #[test]
    fn equal_underlying_data_float_vs_float() {
        assert!(Value::Float(1.5).equal_underlying_data(&Value::Float(1.5)));
        assert!(!Value::Float(1.5).equal_underlying_data(&Value::Float(2.5)));
    }
}
