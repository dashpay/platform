use crate::converter::{js_buffer_to_identifier, js_object_to_fee_refunds};
use drive::fee::result::refunds::{CreditsPerEpochByIdentifier, FeeRefunds};
use drive::fee::result::FeeResult;
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
        let js_fee_refunds = cx.argument::<JsArray>(2)?.to_vec(&mut cx)?;

        let mut credits_per_epoch_by_identifier = CreditsPerEpochByIdentifier::new();
        for item in js_fee_refunds {
            let js_refunds = item.downcast_or_throw::<JsObject, _>(&mut cx)?;

            let js_identifier: Handle<JsBuffer> = js_refunds.get(&mut cx, "identifier")?;
            let identifier = js_buffer_to_identifier(&mut cx, js_identifier)?;

            let js_credits_per_epoch: Handle<JsObject> =
                js_refunds.get(&mut cx, "creditsPerEpoch")?;

            let credits_per_epoch = js_object_to_fee_refunds(&mut cx, js_credits_per_epoch)?;

            credits_per_epoch_by_identifier.insert(identifier, credits_per_epoch);
        }

        let fee_result = FeeResult {
            storage_fee,
            processing_fee,
            fee_refunds: FeeRefunds(credits_per_epoch_by_identifier),
            ..Default::default()
        };

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

        let fee_result_to_add = FeeResult::default_with_fees(storage_fee, processing_fee);

        fee_result_sum
            .checked_add_assign(fee_result_to_add)
            .or_else(|err| cx.throw_error(err.to_string()))?;

        Ok(cx.boxed(Self::new(fee_result_sum)))
    }

    pub fn get_refunds(mut cx: FunctionContext) -> JsResult<JsArray> {
        let fee_result_wrapper_self = cx
            .this()
            .downcast_or_throw::<JsBox<FeeResultWrapper>, _>(&mut cx)?;

        // Clone fee result because IntMap doesn't implement iterator for reference
        let fee_result = fee_result_wrapper_self.deref().deref().deref().clone();

        let js_fee_refunds: Handle<JsArray> = cx.empty_array();

        for (index, (identifier, credits_per_epoch)) in
            fee_result.fee_refunds.into_iter().enumerate()
        {
            let js_epoch_index_map = cx.empty_object();

            for (epoch, credits) in credits_per_epoch {
                // TODO: We could miss fees here
                let js_credits = cx.number(credits as f64);

                js_epoch_index_map.set(&mut cx, epoch.to_string().as_str(), js_credits)?;
            }

            let js_identity_to_epochs = cx.empty_object();

            let js_identifier = JsBuffer::external(&mut cx, identifier);

            js_identity_to_epochs.set(&mut cx, "identifier", js_identifier)?;
            js_identity_to_epochs.set(&mut cx, "creditsPerEpoch", js_epoch_index_map)?;

            js_fee_refunds.set(&mut cx, index as u32, js_identity_to_epochs)?;
        }

        Ok(js_fee_refunds)
    }

    pub fn get_refunds_per_epoch(mut cx: FunctionContext) -> JsResult<JsObject> {
        let fee_result_wrapper_self = cx
            .this()
            .downcast_or_throw::<JsBox<FeeResultWrapper>, _>(&mut cx)?;

        // Clone fee result because IntMap doesn't implement iterator for reference
        let fee_result = fee_result_wrapper_self.deref().deref().deref().clone();

        let js_credits_per_epoch = cx.empty_object();

        for (epoch_index, epoch_credits) in fee_result.fee_refunds.sum_per_epoch() {
            // TODO: We could miss fees here
            let js_credits = cx.number(epoch_credits as f64);

            js_credits_per_epoch.set(&mut cx, epoch_index.to_string().as_str(), js_credits)?;
        }

        Ok(js_credits_per_epoch)
    }
}

impl Finalize for FeeResultWrapper {}

impl Deref for FeeResultWrapper {
    type Target = FeeResult;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
