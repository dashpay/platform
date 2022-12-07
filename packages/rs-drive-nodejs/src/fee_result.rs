use drive::fee::FeeResult;
use neon::prelude::*;
use std::ops::Deref;

pub struct FeeResultWrapper(FeeResult);

impl FeeResultWrapper {
    pub fn new(fee_result: FeeResult) -> Self {
        FeeResultWrapper(fee_result)
    }

    pub fn create(mut cx: FunctionContext) -> JsResult<JsBox<FeeResultWrapper>> {
        let storage_fee = cx.argument::<JsNumber>(0)?.value(&mut cx) as u64;
        let processing_fee = cx.argument::<JsNumber>(1)?.value(&mut cx) as u64;

        let fee_result = FeeResult::from_fees(storage_fee, processing_fee);

        Ok(cx.boxed(Self::new(fee_result)))
    }

    pub fn get_storage_fee(mut cx: FunctionContext) -> JsResult<JsNumber> {
        let fee_result_self = cx
            .this()
            .downcast_or_throw::<JsBox<FeeResultWrapper>, _>(&mut cx)?;

        // TODO: We might lose with the conversion
        Ok(cx.number(fee_result_self.0.storage_fee as f64))
    }

    pub fn get_processing_fee(mut cx: FunctionContext) -> JsResult<JsNumber> {
        let fee_result_self = cx
            .this()
            .downcast_or_throw::<JsBox<FeeResultWrapper>, _>(&mut cx)?;

        // TODO: We might lose with the conversion
        Ok(cx.number(fee_result_self.0.processing_fee as f64))
    }

    pub fn add(mut cx: FunctionContext) -> JsResult<JsBox<Self>> {
        let fee_result_wrapper_to_add = cx.argument::<JsBox<Self>>(0)?;

        let fee_result_wrapper_self = cx
            .this()
            .downcast_or_throw::<JsBox<FeeResultWrapper>, _>(&mut cx)?;

        // TODO: Figure out how to get mutable link from JsBox
        let mut fee_result_sum = fee_result_wrapper_self.deref().deref().deref().clone();

        // TODO: To avoid clone we need to be able to pass a reference to
        //   FeeResult#checked_add_assign
        let fee_result_to_add = fee_result_wrapper_to_add.deref().deref().deref().clone();

        fee_result_sum
            .checked_add_assign(fee_result_to_add)
            .or_else(|err| cx.throw_error(err.to_string()))?;

        Ok(cx.boxed(Self::new(fee_result_sum)))
    }

    pub fn add_fees(mut cx: FunctionContext) -> JsResult<JsBox<Self>> {
        let storage_fee = cx.argument::<JsNumber>(0)?.value(&mut cx) as u64;
        let processing_fee = cx.argument::<JsNumber>(1)?.value(&mut cx) as u64;

        let fee_result_wrapper_self = cx
            .this()
            .downcast_or_throw::<JsBox<FeeResultWrapper>, _>(&mut cx)?;

        // TODO: Figure out how to get mutable link from JsBox
        let mut fee_result_sum = fee_result_wrapper_self.deref().deref().deref().clone();

        let fee_result_to_add = FeeResult::from_fees(storage_fee, processing_fee);

        fee_result_sum
            .checked_add_assign(fee_result_to_add)
            .or_else(|err| cx.throw_error(err.to_string()))?;

        Ok(cx.boxed(Self::new(fee_result_sum)))
    }
}

impl Finalize for FeeResultWrapper {}

impl Deref for FeeResultWrapper {
    type Target = FeeResult;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
