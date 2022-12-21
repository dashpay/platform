use drive::fee::credits::Credits;
use drive::fee::epoch::distribution::calculate_storage_fee_distribution_amount_and_leftovers;
use drive::fee::epoch::EpochIndex;
use neon::prelude::*;

pub mod result;

pub fn js_calculate_storage_fee_distribution_amount_and_leftovers(
    mut cx: FunctionContext,
) -> JsResult<JsArray> {
    let js_storage_fees = cx.argument::<JsNumber>(0)?;
    let storage_fees = js_storage_fees.value(&mut cx) as Credits;

    let js_start_epoch_index = cx.argument::<JsNumber>(1)?;

    let start_epoch_index = EpochIndex::try_from(js_start_epoch_index.value(&mut cx) as i64)
        .or_else(|_| cx.throw_range_error("`startEpochIndex` must fit in u16"))?;

    let js_skip_up_to_epoch_index = cx.argument::<JsNumber>(2)?;
    let skip_up_to_epoch_index =
        EpochIndex::try_from(js_skip_up_to_epoch_index.value(&mut cx) as i64)
            .or_else(|_| cx.throw_range_error("`startEpochIndex` must fit in u16"))?;

    let (amount, leftovers) = calculate_storage_fee_distribution_amount_and_leftovers(
        storage_fees,
        start_epoch_index,
        skip_up_to_epoch_index,
    )
    .or_else(|e| cx.throw_error(e.to_string()))?;

    let js_array = cx.empty_array();

    let js_amount = cx.number(amount as f64);
    let js_leftovers = cx.number(leftovers as f64);

    js_array.set(&mut cx, 0, js_amount)?;
    js_array.set(&mut cx, 1, js_leftovers)?;

    Ok(js_array)
}
