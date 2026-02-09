use crate::error::{WasmDppError, WasmDppResult};
use dpp::balances::credits::TokenAmount;
use serde::Deserialize;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const DISTRIBUTION_OPTIONS_TS: &str = r#"
export interface DistributionFixedAmountOptions {
    amount: bigint;
}

export interface DistributionRandomOptions {
    min: bigint;
    max: bigint;
}

export interface DistributionStepDecreasingAmountOptions {
    stepCount: number;
    decreasePerIntervalNumerator: number;
    decreasePerIntervalDenominator: number;
    startDecreasingOffset?: bigint;
    maxIntervalCount?: number;
    distributionStartAmount: bigint;
    trailingDistributionIntervalAmount: bigint;
    minValue?: bigint;
}

export interface DistributionLinearOptions {
    a: bigint;
    d: bigint;
    startStep?: bigint;
    startingAmount: bigint;
    minValue?: bigint;
    maxValue?: bigint;
}

export interface DistributionPolynomialOptions {
    a: bigint;
    d: bigint;
    m: bigint;
    n: bigint;
    o: bigint;
    startMoment?: bigint;
    b: bigint;
    minValue?: bigint;
    maxValue?: bigint;
}

export interface DistributionExponentialOptions {
    a: bigint;
    d: bigint;
    m: bigint;
    n: bigint;
    o: bigint;
    startMoment?: bigint;
    b: bigint;
    minValue?: bigint;
    maxValue?: bigint;
}

export interface DistributionLogarithmicOptions {
    a: bigint;
    d: bigint;
    m: bigint;
    n: bigint;
    o: bigint;
    startMoment?: bigint;
    b: bigint;
    minValue?: bigint;
    maxValue?: bigint;
}

export interface DistributionInvertedLogarithmicOptions {
    a: bigint;
    d: bigint;
    m: bigint;
    n: bigint;
    o: bigint;
    startMoment?: bigint;
    b: bigint;
    minValue?: bigint;
    maxValue?: bigint;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "DistributionFixedAmountOptions")]
    pub type DistributionFixedAmountOptionsJs;

    #[wasm_bindgen(typescript_type = "DistributionRandomOptions")]
    pub type DistributionRandomOptionsJs;

    #[wasm_bindgen(typescript_type = "DistributionStepDecreasingAmountOptions")]
    pub type DistributionStepDecreasingAmountOptionsJs;

    #[wasm_bindgen(typescript_type = "DistributionLinearOptions")]
    pub type DistributionLinearOptionsJs;

    #[wasm_bindgen(typescript_type = "DistributionPolynomialOptions")]
    pub type DistributionPolynomialOptionsJs;

    #[wasm_bindgen(typescript_type = "DistributionExponentialOptions")]
    pub type DistributionExponentialOptionsJs;

    #[wasm_bindgen(typescript_type = "DistributionLogarithmicOptions")]
    pub type DistributionLogarithmicOptionsJs;

    #[wasm_bindgen(typescript_type = "DistributionInvertedLogarithmicOptions")]
    pub type DistributionInvertedLogarithmicOptionsJs;
}

// DistributionFixedAmount

#[wasm_bindgen(js_name = "DistributionFixedAmount")]
pub struct DistributionFixedAmountWasm {
    pub(crate) amount: TokenAmount,
}

#[wasm_bindgen(js_class = DistributionFixedAmount)]
impl DistributionFixedAmountWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(options: DistributionFixedAmountOptionsJs) -> WasmDppResult<Self> {
        #[derive(Deserialize)]
        struct Options {
            amount: TokenAmount,
        }
        let opts: Options = serde_wasm_bindgen::from_value(options.into())
            .map_err(|e| WasmDppError::invalid_argument(e.to_string()))?;
        Ok(Self {
            amount: opts.amount,
        })
    }

    #[wasm_bindgen(getter)]
    pub fn amount(&self) -> TokenAmount {
        self.amount
    }

    #[wasm_bindgen(setter)]
    pub fn set_amount(&mut self, value: TokenAmount) {
        self.amount = value;
    }
}

// DistributionRandom

#[wasm_bindgen(js_name = "DistributionRandom")]
pub struct DistributionRandomWasm {
    pub(crate) min: TokenAmount,
    pub(crate) max: TokenAmount,
}

#[wasm_bindgen(js_class = DistributionRandom)]
impl DistributionRandomWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(options: DistributionRandomOptionsJs) -> WasmDppResult<Self> {
        #[derive(Deserialize)]
        struct Options {
            min: TokenAmount,
            max: TokenAmount,
        }
        let opts: Options = serde_wasm_bindgen::from_value(options.into())
            .map_err(|e| WasmDppError::invalid_argument(e.to_string()))?;
        Ok(Self {
            min: opts.min,
            max: opts.max,
        })
    }

    #[wasm_bindgen(getter)]
    pub fn min(&self) -> TokenAmount {
        self.min
    }

    #[wasm_bindgen(setter)]
    pub fn set_min(&mut self, value: TokenAmount) {
        self.min = value;
    }

    #[wasm_bindgen(getter)]
    pub fn max(&self) -> TokenAmount {
        self.max
    }

    #[wasm_bindgen(setter)]
    pub fn set_max(&mut self, value: TokenAmount) {
        self.max = value;
    }
}

// DistributionStepDecreasingAmount

#[wasm_bindgen(js_name = "DistributionStepDecreasingAmount")]
pub struct DistributionStepDecreasingAmountWasm {
    pub(crate) step_count: u32,
    pub(crate) decrease_per_interval_numerator: u16,
    pub(crate) decrease_per_interval_denominator: u16,
    pub(crate) start_decreasing_offset: Option<u64>,
    pub(crate) max_interval_count: Option<u16>,
    pub(crate) distribution_start_amount: TokenAmount,
    pub(crate) trailing_distribution_interval_amount: TokenAmount,
    pub(crate) min_value: Option<u64>,
}

#[wasm_bindgen(js_class = DistributionStepDecreasingAmount)]
impl DistributionStepDecreasingAmountWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(options: DistributionStepDecreasingAmountOptionsJs) -> WasmDppResult<Self> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Options {
            step_count: u32,
            decrease_per_interval_numerator: u16,
            decrease_per_interval_denominator: u16,
            #[serde(default)]
            start_decreasing_offset: Option<u64>,
            #[serde(default)]
            max_interval_count: Option<u16>,
            distribution_start_amount: TokenAmount,
            trailing_distribution_interval_amount: TokenAmount,
            #[serde(default)]
            min_value: Option<u64>,
        }
        let opts: Options = serde_wasm_bindgen::from_value(options.into())
            .map_err(|e| WasmDppError::invalid_argument(e.to_string()))?;
        Ok(Self {
            step_count: opts.step_count,
            decrease_per_interval_numerator: opts.decrease_per_interval_numerator,
            decrease_per_interval_denominator: opts.decrease_per_interval_denominator,
            start_decreasing_offset: opts.start_decreasing_offset,
            max_interval_count: opts.max_interval_count,
            distribution_start_amount: opts.distribution_start_amount,
            trailing_distribution_interval_amount: opts.trailing_distribution_interval_amount,
            min_value: opts.min_value,
        })
    }

    #[wasm_bindgen(getter = "stepCount")]
    pub fn step_count(&self) -> u32 {
        self.step_count
    }

    #[wasm_bindgen(setter = "stepCount")]
    pub fn set_step_count(&mut self, value: u32) {
        self.step_count = value;
    }

    #[wasm_bindgen(getter = "decreasePerIntervalNumerator")]
    pub fn decrease_per_interval_numerator(&self) -> u16 {
        self.decrease_per_interval_numerator
    }

    #[wasm_bindgen(setter = "decreasePerIntervalNumerator")]
    pub fn set_decrease_per_interval_numerator(&mut self, value: u16) {
        self.decrease_per_interval_numerator = value;
    }

    #[wasm_bindgen(getter = "decreasePerIntervalDenominator")]
    pub fn decrease_per_interval_denominator(&self) -> u16 {
        self.decrease_per_interval_denominator
    }

    #[wasm_bindgen(setter = "decreasePerIntervalDenominator")]
    pub fn set_decrease_per_interval_denominator(&mut self, value: u16) {
        self.decrease_per_interval_denominator = value;
    }

    #[wasm_bindgen(getter = "startDecreasingOffset")]
    pub fn start_decreasing_offset(&self) -> Option<u64> {
        self.start_decreasing_offset
    }

    #[wasm_bindgen(setter = "startDecreasingOffset")]
    pub fn set_start_decreasing_offset(&mut self, value: Option<u64>) {
        self.start_decreasing_offset = value;
    }

    #[wasm_bindgen(getter = "maxIntervalCount")]
    pub fn max_interval_count(&self) -> Option<u16> {
        self.max_interval_count
    }

    #[wasm_bindgen(setter = "maxIntervalCount")]
    pub fn set_max_interval_count(&mut self, value: Option<u16>) {
        self.max_interval_count = value;
    }

    #[wasm_bindgen(getter = "distributionStartAmount")]
    pub fn distribution_start_amount(&self) -> TokenAmount {
        self.distribution_start_amount
    }

    #[wasm_bindgen(setter = "distributionStartAmount")]
    pub fn set_distribution_start_amount(&mut self, value: TokenAmount) {
        self.distribution_start_amount = value;
    }

    #[wasm_bindgen(getter = "trailingDistributionIntervalAmount")]
    pub fn trailing_distribution_interval_amount(&self) -> TokenAmount {
        self.trailing_distribution_interval_amount
    }

    #[wasm_bindgen(setter = "trailingDistributionIntervalAmount")]
    pub fn set_trailing_distribution_interval_amount(&mut self, value: TokenAmount) {
        self.trailing_distribution_interval_amount = value;
    }

    #[wasm_bindgen(getter = "minValue")]
    pub fn min_value(&self) -> Option<u64> {
        self.min_value
    }

    #[wasm_bindgen(setter = "minValue")]
    pub fn set_min_value(&mut self, value: Option<u64>) {
        self.min_value = value;
    }
}

// DistributionLinear

#[wasm_bindgen(js_name = "DistributionLinear")]
pub struct DistributionLinearWasm {
    pub(crate) a: i64,
    pub(crate) d: u64,
    pub(crate) start_step: Option<u64>,
    pub(crate) starting_amount: TokenAmount,
    pub(crate) min_value: Option<u64>,
    pub(crate) max_value: Option<u64>,
}

#[wasm_bindgen(js_class = DistributionLinear)]
impl DistributionLinearWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(options: DistributionLinearOptionsJs) -> WasmDppResult<Self> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Options {
            a: i64,
            d: u64,
            #[serde(default)]
            start_step: Option<u64>,
            starting_amount: TokenAmount,
            #[serde(default)]
            min_value: Option<u64>,
            #[serde(default)]
            max_value: Option<u64>,
        }
        let opts: Options = serde_wasm_bindgen::from_value(options.into())
            .map_err(|e| WasmDppError::invalid_argument(e.to_string()))?;
        Ok(Self {
            a: opts.a,
            d: opts.d,
            start_step: opts.start_step,
            starting_amount: opts.starting_amount,
            min_value: opts.min_value,
            max_value: opts.max_value,
        })
    }

    #[wasm_bindgen(getter)]
    pub fn a(&self) -> i64 {
        self.a
    }

    #[wasm_bindgen(setter)]
    pub fn set_a(&mut self, value: i64) {
        self.a = value;
    }

    #[wasm_bindgen(getter)]
    pub fn d(&self) -> u64 {
        self.d
    }

    #[wasm_bindgen(setter)]
    pub fn set_d(&mut self, value: u64) {
        self.d = value;
    }

    #[wasm_bindgen(getter = "startStep")]
    pub fn start_step(&self) -> Option<u64> {
        self.start_step
    }

    #[wasm_bindgen(setter = "startStep")]
    pub fn set_start_step(&mut self, value: Option<u64>) {
        self.start_step = value;
    }

    #[wasm_bindgen(getter = "startingAmount")]
    pub fn starting_amount(&self) -> TokenAmount {
        self.starting_amount
    }

    #[wasm_bindgen(setter = "startingAmount")]
    pub fn set_starting_amount(&mut self, value: TokenAmount) {
        self.starting_amount = value;
    }

    #[wasm_bindgen(getter = "minValue")]
    pub fn min_value(&self) -> Option<u64> {
        self.min_value
    }

    #[wasm_bindgen(setter = "minValue")]
    pub fn set_min_value(&mut self, value: Option<u64>) {
        self.min_value = value;
    }

    #[wasm_bindgen(getter = "maxValue")]
    pub fn max_value(&self) -> Option<u64> {
        self.max_value
    }

    #[wasm_bindgen(setter = "maxValue")]
    pub fn set_max_value(&mut self, value: Option<u64>) {
        self.max_value = value;
    }
}

// DistributionPolynomial

#[wasm_bindgen(js_name = "DistributionPolynomial")]
pub struct DistributionPolynomialWasm {
    pub(crate) a: i64,
    pub(crate) d: u64,
    pub(crate) m: i64,
    pub(crate) n: u64,
    pub(crate) o: i64,
    pub(crate) start_moment: Option<u64>,
    pub(crate) b: TokenAmount,
    pub(crate) min_value: Option<u64>,
    pub(crate) max_value: Option<u64>,
}

#[wasm_bindgen(js_class = DistributionPolynomial)]
impl DistributionPolynomialWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(options: DistributionPolynomialOptionsJs) -> WasmDppResult<Self> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Options {
            a: i64,
            d: u64,
            m: i64,
            n: u64,
            o: i64,
            #[serde(default)]
            start_moment: Option<u64>,
            b: TokenAmount,
            #[serde(default)]
            min_value: Option<u64>,
            #[serde(default)]
            max_value: Option<u64>,
        }
        let opts: Options = serde_wasm_bindgen::from_value(options.into())
            .map_err(|e| WasmDppError::invalid_argument(e.to_string()))?;
        Ok(Self {
            a: opts.a,
            d: opts.d,
            m: opts.m,
            n: opts.n,
            o: opts.o,
            start_moment: opts.start_moment,
            b: opts.b,
            min_value: opts.min_value,
            max_value: opts.max_value,
        })
    }

    #[wasm_bindgen(getter)]
    pub fn a(&self) -> i64 {
        self.a
    }

    #[wasm_bindgen(setter)]
    pub fn set_a(&mut self, value: i64) {
        self.a = value;
    }

    #[wasm_bindgen(getter)]
    pub fn d(&self) -> u64 {
        self.d
    }

    #[wasm_bindgen(setter)]
    pub fn set_d(&mut self, value: u64) {
        self.d = value;
    }

    #[wasm_bindgen(getter)]
    pub fn m(&self) -> i64 {
        self.m
    }

    #[wasm_bindgen(setter)]
    pub fn set_m(&mut self, value: i64) {
        self.m = value;
    }

    #[wasm_bindgen(getter)]
    pub fn n(&self) -> u64 {
        self.n
    }

    #[wasm_bindgen(setter)]
    pub fn set_n(&mut self, value: u64) {
        self.n = value;
    }

    #[wasm_bindgen(getter)]
    pub fn o(&self) -> i64 {
        self.o
    }

    #[wasm_bindgen(setter)]
    pub fn set_o(&mut self, value: i64) {
        self.o = value;
    }

    #[wasm_bindgen(getter = "startMoment")]
    pub fn start_moment(&self) -> Option<u64> {
        self.start_moment
    }

    #[wasm_bindgen(setter = "startMoment")]
    pub fn set_start_moment(&mut self, value: Option<u64>) {
        self.start_moment = value;
    }

    #[wasm_bindgen(getter)]
    pub fn b(&self) -> TokenAmount {
        self.b
    }

    #[wasm_bindgen(setter)]
    pub fn set_b(&mut self, value: TokenAmount) {
        self.b = value;
    }

    #[wasm_bindgen(getter = "minValue")]
    pub fn min_value(&self) -> Option<u64> {
        self.min_value
    }

    #[wasm_bindgen(setter = "minValue")]
    pub fn set_min_value(&mut self, value: Option<u64>) {
        self.min_value = value;
    }

    #[wasm_bindgen(getter = "maxValue")]
    pub fn max_value(&self) -> Option<u64> {
        self.max_value
    }

    #[wasm_bindgen(setter = "maxValue")]
    pub fn set_max_value(&mut self, value: Option<u64>) {
        self.max_value = value;
    }
}

// DistributionExponential

#[wasm_bindgen(js_name = "DistributionExponential")]
pub struct DistributionExponentialWasm {
    pub(crate) a: u64,
    pub(crate) d: u64,
    pub(crate) m: i64,
    pub(crate) n: u64,
    pub(crate) o: i64,
    pub(crate) start_moment: Option<u64>,
    pub(crate) b: TokenAmount,
    pub(crate) min_value: Option<u64>,
    pub(crate) max_value: Option<u64>,
}

#[wasm_bindgen(js_class = DistributionExponential)]
impl DistributionExponentialWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(options: DistributionExponentialOptionsJs) -> WasmDppResult<Self> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Options {
            a: u64,
            d: u64,
            m: i64,
            n: u64,
            o: i64,
            #[serde(default)]
            start_moment: Option<u64>,
            b: TokenAmount,
            #[serde(default)]
            min_value: Option<u64>,
            #[serde(default)]
            max_value: Option<u64>,
        }
        let opts: Options = serde_wasm_bindgen::from_value(options.into())
            .map_err(|e| WasmDppError::invalid_argument(e.to_string()))?;
        Ok(Self {
            a: opts.a,
            d: opts.d,
            m: opts.m,
            n: opts.n,
            o: opts.o,
            start_moment: opts.start_moment,
            b: opts.b,
            min_value: opts.min_value,
            max_value: opts.max_value,
        })
    }

    #[wasm_bindgen(getter)]
    pub fn a(&self) -> u64 {
        self.a
    }

    #[wasm_bindgen(setter)]
    pub fn set_a(&mut self, value: u64) {
        self.a = value;
    }

    #[wasm_bindgen(getter)]
    pub fn d(&self) -> u64 {
        self.d
    }

    #[wasm_bindgen(setter)]
    pub fn set_d(&mut self, value: u64) {
        self.d = value;
    }

    #[wasm_bindgen(getter)]
    pub fn m(&self) -> i64 {
        self.m
    }

    #[wasm_bindgen(setter)]
    pub fn set_m(&mut self, value: i64) {
        self.m = value;
    }

    #[wasm_bindgen(getter)]
    pub fn n(&self) -> u64 {
        self.n
    }

    #[wasm_bindgen(setter)]
    pub fn set_n(&mut self, value: u64) {
        self.n = value;
    }

    #[wasm_bindgen(getter)]
    pub fn o(&self) -> i64 {
        self.o
    }

    #[wasm_bindgen(setter)]
    pub fn set_o(&mut self, value: i64) {
        self.o = value;
    }

    #[wasm_bindgen(getter = "startMoment")]
    pub fn start_moment(&self) -> Option<u64> {
        self.start_moment
    }

    #[wasm_bindgen(setter = "startMoment")]
    pub fn set_start_moment(&mut self, value: Option<u64>) {
        self.start_moment = value;
    }

    #[wasm_bindgen(getter)]
    pub fn b(&self) -> TokenAmount {
        self.b
    }

    #[wasm_bindgen(setter)]
    pub fn set_b(&mut self, value: TokenAmount) {
        self.b = value;
    }

    #[wasm_bindgen(getter = "minValue")]
    pub fn min_value(&self) -> Option<u64> {
        self.min_value
    }

    #[wasm_bindgen(setter = "minValue")]
    pub fn set_min_value(&mut self, value: Option<u64>) {
        self.min_value = value;
    }

    #[wasm_bindgen(getter = "maxValue")]
    pub fn max_value(&self) -> Option<u64> {
        self.max_value
    }

    #[wasm_bindgen(setter = "maxValue")]
    pub fn set_max_value(&mut self, value: Option<u64>) {
        self.max_value = value;
    }
}

// DistributionLogarithmic

#[wasm_bindgen(js_name = "DistributionLogarithmic")]
pub struct DistributionLogarithmicWasm {
    pub(crate) a: i64,
    pub(crate) d: u64,
    pub(crate) m: u64,
    pub(crate) n: u64,
    pub(crate) o: i64,
    pub(crate) start_moment: Option<u64>,
    pub(crate) b: TokenAmount,
    pub(crate) min_value: Option<u64>,
    pub(crate) max_value: Option<u64>,
}

#[wasm_bindgen(js_class = DistributionLogarithmic)]
impl DistributionLogarithmicWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(options: DistributionLogarithmicOptionsJs) -> WasmDppResult<Self> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Options {
            a: i64,
            d: u64,
            m: u64,
            n: u64,
            o: i64,
            #[serde(default)]
            start_moment: Option<u64>,
            b: TokenAmount,
            #[serde(default)]
            min_value: Option<u64>,
            #[serde(default)]
            max_value: Option<u64>,
        }
        let opts: Options = serde_wasm_bindgen::from_value(options.into())
            .map_err(|e| WasmDppError::invalid_argument(e.to_string()))?;
        Ok(Self {
            a: opts.a,
            d: opts.d,
            m: opts.m,
            n: opts.n,
            o: opts.o,
            start_moment: opts.start_moment,
            b: opts.b,
            min_value: opts.min_value,
            max_value: opts.max_value,
        })
    }

    #[wasm_bindgen(getter)]
    pub fn a(&self) -> i64 {
        self.a
    }

    #[wasm_bindgen(setter)]
    pub fn set_a(&mut self, value: i64) {
        self.a = value;
    }

    #[wasm_bindgen(getter)]
    pub fn d(&self) -> u64 {
        self.d
    }

    #[wasm_bindgen(setter)]
    pub fn set_d(&mut self, value: u64) {
        self.d = value;
    }

    #[wasm_bindgen(getter)]
    pub fn m(&self) -> u64 {
        self.m
    }

    #[wasm_bindgen(setter)]
    pub fn set_m(&mut self, value: u64) {
        self.m = value;
    }

    #[wasm_bindgen(getter)]
    pub fn n(&self) -> u64 {
        self.n
    }

    #[wasm_bindgen(setter)]
    pub fn set_n(&mut self, value: u64) {
        self.n = value;
    }

    #[wasm_bindgen(getter)]
    pub fn o(&self) -> i64 {
        self.o
    }

    #[wasm_bindgen(setter)]
    pub fn set_o(&mut self, value: i64) {
        self.o = value;
    }

    #[wasm_bindgen(getter = "startMoment")]
    pub fn start_moment(&self) -> Option<u64> {
        self.start_moment
    }

    #[wasm_bindgen(setter = "startMoment")]
    pub fn set_start_moment(&mut self, value: Option<u64>) {
        self.start_moment = value;
    }

    #[wasm_bindgen(getter)]
    pub fn b(&self) -> TokenAmount {
        self.b
    }

    #[wasm_bindgen(setter)]
    pub fn set_b(&mut self, value: TokenAmount) {
        self.b = value;
    }

    #[wasm_bindgen(getter = "minValue")]
    pub fn min_value(&self) -> Option<u64> {
        self.min_value
    }

    #[wasm_bindgen(setter = "minValue")]
    pub fn set_min_value(&mut self, value: Option<u64>) {
        self.min_value = value;
    }

    #[wasm_bindgen(getter = "maxValue")]
    pub fn max_value(&self) -> Option<u64> {
        self.max_value
    }

    #[wasm_bindgen(setter = "maxValue")]
    pub fn set_max_value(&mut self, value: Option<u64>) {
        self.max_value = value;
    }
}

// DistributionInvertedLogarithmic

#[wasm_bindgen(js_name = "DistributionInvertedLogarithmic")]
pub struct DistributionInvertedLogarithmicWasm {
    pub(crate) a: i64,
    pub(crate) d: u64,
    pub(crate) m: u64,
    pub(crate) n: u64,
    pub(crate) o: i64,
    pub(crate) start_moment: Option<u64>,
    pub(crate) b: TokenAmount,
    pub(crate) min_value: Option<u64>,
    pub(crate) max_value: Option<u64>,
}

#[wasm_bindgen(js_class = DistributionInvertedLogarithmic)]
impl DistributionInvertedLogarithmicWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(options: DistributionInvertedLogarithmicOptionsJs) -> WasmDppResult<Self> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Options {
            a: i64,
            d: u64,
            m: u64,
            n: u64,
            o: i64,
            #[serde(default)]
            start_moment: Option<u64>,
            b: TokenAmount,
            #[serde(default)]
            min_value: Option<u64>,
            #[serde(default)]
            max_value: Option<u64>,
        }
        let opts: Options = serde_wasm_bindgen::from_value(options.into())
            .map_err(|e| WasmDppError::invalid_argument(e.to_string()))?;
        Ok(Self {
            a: opts.a,
            d: opts.d,
            m: opts.m,
            n: opts.n,
            o: opts.o,
            start_moment: opts.start_moment,
            b: opts.b,
            min_value: opts.min_value,
            max_value: opts.max_value,
        })
    }

    #[wasm_bindgen(getter)]
    pub fn a(&self) -> i64 {
        self.a
    }

    #[wasm_bindgen(setter)]
    pub fn set_a(&mut self, value: i64) {
        self.a = value;
    }

    #[wasm_bindgen(getter)]
    pub fn d(&self) -> u64 {
        self.d
    }

    #[wasm_bindgen(setter)]
    pub fn set_d(&mut self, value: u64) {
        self.d = value;
    }

    #[wasm_bindgen(getter)]
    pub fn m(&self) -> u64 {
        self.m
    }

    #[wasm_bindgen(setter)]
    pub fn set_m(&mut self, value: u64) {
        self.m = value;
    }

    #[wasm_bindgen(getter)]
    pub fn n(&self) -> u64 {
        self.n
    }

    #[wasm_bindgen(setter)]
    pub fn set_n(&mut self, value: u64) {
        self.n = value;
    }

    #[wasm_bindgen(getter)]
    pub fn o(&self) -> i64 {
        self.o
    }

    #[wasm_bindgen(setter)]
    pub fn set_o(&mut self, value: i64) {
        self.o = value;
    }

    #[wasm_bindgen(getter = "startMoment")]
    pub fn start_moment(&self) -> Option<u64> {
        self.start_moment
    }

    #[wasm_bindgen(setter = "startMoment")]
    pub fn set_start_moment(&mut self, value: Option<u64>) {
        self.start_moment = value;
    }

    #[wasm_bindgen(getter)]
    pub fn b(&self) -> TokenAmount {
        self.b
    }

    #[wasm_bindgen(setter)]
    pub fn set_b(&mut self, value: TokenAmount) {
        self.b = value;
    }

    #[wasm_bindgen(getter = "minValue")]
    pub fn min_value(&self) -> Option<u64> {
        self.min_value
    }

    #[wasm_bindgen(setter = "minValue")]
    pub fn set_min_value(&mut self, value: Option<u64>) {
        self.min_value = value;
    }

    #[wasm_bindgen(getter = "maxValue")]
    pub fn max_value(&self) -> Option<u64> {
        self.max_value
    }

    #[wasm_bindgen(setter = "maxValue")]
    pub fn set_max_value(&mut self, value: Option<u64>) {
        self.max_value = value;
    }
}
