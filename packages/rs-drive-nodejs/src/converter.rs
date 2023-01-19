use drive::dpp::identity::{IdentityPublicKey, KeyID, KeyType, Purpose, SecurityLevel};
use drive::drive::block_info::BlockInfo;
use drive::drive::flags::StorageFlags;
use drive::fee::credits::Credits;
use drive::fee::epoch::CreditsPerEpoch;
use drive::fee_pools::epochs::Epoch;
use drive::grovedb::reference_path::ReferencePathType;
use drive::grovedb::{Element, PathQuery, Query, SizedQuery};
use neon::prelude::*;
use neon::types::buffer::TypedArray;
use num::FromPrimitive;
use std::borrow::Borrow;
use std::num::ParseIntError;

fn element_to_string(element: &Element) -> &'static str {
    match element {
        Element::Item(..) => "item",
        Element::SumItem(..) => "sumItem",
        Element::Reference(..) => "reference",
        Element::Tree(..) => "tree",
        Element::SumTree(..) => "sumTree",
    }
}

pub fn js_object_to_element<'a, C: Context<'a>>(
    cx: &mut C,
    js_object: Handle<JsObject>,
) -> NeonResult<Element> {
    let js_element_type: Handle<JsString> = js_object.get(cx, "type")?;

    let element_type: String = js_element_type.value(cx);

    let js_element_epoch: Option<Handle<JsNumber>> = js_object.get_opt(cx, "epoch")?;

    let element_flags = if let Some(js_epoch) = js_element_epoch {
        let epoch = u16::try_from(js_epoch.value(cx) as i64)
            .or_else(|_| cx.throw_range_error("`epochs` must fit in u16"))?;

        let js_maybe_owner_id: Option<Handle<JsBuffer>> = js_object.get_opt(cx, "ownerId")?;

        let maybe_owner_id = js_maybe_owner_id
            .map(|js_buffer| js_buffer_to_identifier(cx, js_buffer))
            .transpose()?;

        let storage_flags = StorageFlags::new_single_epoch(epoch, maybe_owner_id);

        storage_flags.to_some_element_flags()
    } else {
        None
    };

    match element_type.as_str() {
        "item" => {
            let js_buffer: Handle<JsBuffer> = js_object.get(cx, "value")?;
            let item = js_buffer_to_vec_u8(cx, js_buffer);

            Ok(Element::new_item_with_flags(item, element_flags))
        }
        "reference" => {
            let js_object: Handle<JsObject> = js_object.get(cx, "value")?;
            let reference = js_object_to_reference(cx, js_object)?;

            Ok(Element::new_reference_with_flags(reference, element_flags))
        }
        "tree" => Ok(Element::empty_tree_with_flags(element_flags)),
        _ => cx.throw_error(format!("Unexpected element type {}", element_type)),
    }
}

fn js_object_to_reference<'a, C: Context<'a>>(
    cx: &mut C,
    js_object: Handle<JsObject>,
) -> NeonResult<ReferencePathType> {
    let js_reference_type: Handle<JsString> = js_object.get(cx, "type")?;
    let reference_type: String = js_reference_type.value(cx);

    match reference_type.as_str() {
        "absolutePathReference" => {
            let js_path: Handle<JsArray> = js_object.get(cx, "path")?;
            let path = js_array_of_buffers_to_vec(cx, js_path)?;

            Ok(ReferencePathType::AbsolutePathReference(path))
        }
        "upstreamRootHeightReference" => {
            let js_path: Handle<JsArray> = js_object.get(cx, "path")?;
            let path = js_array_of_buffers_to_vec(cx, js_path)?;

            let js_relativity_index: Handle<JsNumber> = js_object.get(cx, "relativityIndex")?;
            let relativity_index_f64: f64 = js_relativity_index.value(cx);
            let relativity_index_option: Option<u8> = FromPrimitive::from_f64(relativity_index_f64);
            let relativity_index: u8 = relativity_index_option
                .ok_or(())
                .or_else(|_| cx.throw_error("cannot convert relativity_index from f64 to u8"))?;

            Ok(ReferencePathType::UpstreamRootHeightReference(
                relativity_index,
                path,
            ))
        }
        "upstreamFromElementHeightReference" => {
            let js_path: Handle<JsArray> = js_object.get(cx, "path")?;
            let path = js_array_of_buffers_to_vec(cx, js_path)?;

            let js_relativity_index: Handle<JsNumber> = js_object.get(cx, "relativityIndex")?;
            let relativity_index_f64: f64 = js_relativity_index.value(cx);
            let relativity_index_option: Option<u8> = FromPrimitive::from_f64(relativity_index_f64);
            let relativity_index: u8 = relativity_index_option
                .ok_or(())
                .or_else(|_| cx.throw_error("cannot convert relativity_index from f64 to u8"))?;

            Ok(ReferencePathType::UpstreamFromElementHeightReference(
                relativity_index,
                path,
            ))
        }
        "cousinReference" => {
            let js_key: Handle<JsBuffer> = js_object.get(cx, "key")?;
            let key = js_buffer_to_vec_u8(cx, js_key);

            Ok(ReferencePathType::CousinReference(key))
        }
        "siblingReference" => {
            let js_key: Handle<JsBuffer> = js_object.get(cx, "key")?;
            let key = js_buffer_to_vec_u8(cx, js_key);

            Ok(ReferencePathType::SiblingReference(key))
        }
        _ => cx.throw_error(format!("Unexpected reference type {}", reference_type)),
    }
}

pub fn element_to_js_object<'a, C: Context<'a>>(
    cx: &mut C,
    element: Element,
) -> NeonResult<Handle<'a, JsValue>> {
    let js_object = cx.empty_object();
    let js_type_string = cx.string(element_to_string(&element));
    js_object.set(cx, "type", js_type_string)?;

    let maybe_js_value: Option<Handle<JsValue>> = match element {
        Element::Item(item, _) => {
            let js_buffer = JsBuffer::external(cx, item);
            Some(js_buffer.upcast())
        }
        Element::SumItem(number, _) => {
            let js_number = cx.number(number as f64).upcast();
            Some(js_number)
        }
        Element::Reference(reference, _, _) => {
            let reference = reference_to_dictionary(cx, reference)?;
            Some(reference)
        }
        Element::Tree(Some(tree), _) | Element::SumTree(Some(tree), ..) => {
            let js_buffer = JsBuffer::external(cx, tree);
            Some(js_buffer.upcast())
        }
        Element::Tree(None, _) | Element::SumTree(None, ..) => None,
    };

    if let Some(js_value) = maybe_js_value {
        js_object.set(cx, "value", js_value)?;
    }

    Ok(js_object.upcast())
}

pub fn nested_vecs_to_js<'a, C: Context<'a>>(
    cx: &mut C,
    v: Vec<Vec<u8>>,
) -> NeonResult<Handle<'a, JsValue>> {
    let js_array: Handle<JsArray> = cx.empty_array();

    for (index, bytes) in v.iter().enumerate() {
        let js_buffer = JsBuffer::external(cx, bytes.clone());
        let js_value = js_buffer.as_value(cx);
        js_array.set(cx, index as u32, js_value)?;
    }

    Ok(js_array.upcast())
}

pub fn reference_to_dictionary<'a, C: Context<'a>>(
    cx: &mut C,
    reference: ReferencePathType,
) -> NeonResult<Handle<'a, JsValue>> {
    let js_object: Handle<JsObject> = cx.empty_object();

    match reference {
        ReferencePathType::AbsolutePathReference(path) => {
            let js_type_name = cx.string("absolutePathReference");
            let js_path = nested_vecs_to_js(cx, path)?;

            js_object.set(cx, "type", js_type_name)?;
            js_object.set(cx, "path", js_path)?;
        }
        ReferencePathType::UpstreamRootHeightReference(relativity_index, path) => {
            let js_type_name = cx.string("upstreamRootHeightReference");
            let js_relativity_index = cx.number(relativity_index);
            let js_path = nested_vecs_to_js(cx, path)?;

            js_object.set(cx, "type", js_type_name)?;
            js_object.set(cx, "relativityIndex", js_relativity_index)?;
            js_object.set(cx, "path", js_path)?;
        }
        ReferencePathType::UpstreamFromElementHeightReference(relativity_index, path) => {
            let js_type_name = cx.string("upstreamFromElementHeightReference");
            let js_relativity_index = cx.number(relativity_index);
            let js_path = nested_vecs_to_js(cx, path)?;

            js_object.set(cx, "type", js_type_name)?;
            js_object.set(cx, "relativityIndex", js_relativity_index)?;
            js_object.set(cx, "path", js_path)?;
        }
        ReferencePathType::CousinReference(key) => {
            let js_type_name = cx.string("cousinReference");
            let js_key = JsBuffer::external(cx, key);

            js_object.set(cx, "type", js_type_name)?;
            js_object.set(cx, "key", js_key)?;
        }
        ReferencePathType::SiblingReference(key) => {
            let js_type_name = cx.string("siblingReference");
            let js_key = JsBuffer::external(cx, key);

            js_object.set(cx, "type", js_type_name)?;
            js_object.set(cx, "key", js_key)?;
        }
        ReferencePathType::RemovedCousinReference(path) => {
            let js_type_name = cx.string("removedCousinReference");
            let js_path = nested_vecs_to_js(cx, path)?;

            js_object.set(cx, "type", js_type_name)?;
            js_object.set(cx, "path", js_path)?;
        }
    }

    Ok(js_object.upcast())
}

pub fn js_buffer_to_identifier<'a, C: Context<'a>>(
    cx: &mut C,
    js_buffer: Handle<JsBuffer>,
) -> NeonResult<[u8; 32]> {
    // let guard = cx.lock();

    let key_memory_view = js_buffer.borrow();

    // let key_buffer = js_buffer.deref();
    // let key_memory_view = js_buffer.borrow(&guard);
    let key_slice: &[u8] = key_memory_view.as_slice(cx);
    <[u8; 32]>::try_from(key_slice).or_else(|_| cx.throw_type_error("hash must be 32 bytes long"))
}

pub fn js_buffer_to_vec_u8<'a, C: Context<'a>>(cx: &mut C, js_buffer: Handle<JsBuffer>) -> Vec<u8> {
    // let guard = cx.lock();

    let key_memory_view = js_buffer.borrow();

    // let key_buffer = js_buffer.deref();
    // let key_memory_view = js_buffer.borrow(&guard);
    let key_slice: &[u8] = key_memory_view.as_slice(cx);
    key_slice.to_vec()
}

pub fn js_array_of_buffers_to_vec<'a, C: Context<'a>>(
    cx: &mut C,
    js_array: Handle<JsArray>,
) -> NeonResult<Vec<Vec<u8>>> {
    let buf_vec = js_array.to_vec(cx)?;
    let mut vec: Vec<Vec<u8>> = Vec::with_capacity(buf_vec.len());

    for buf in buf_vec {
        let js_buffer_handle = buf.downcast_or_throw::<JsBuffer, _>(cx)?;
        vec.push(js_buffer_to_vec_u8(cx, js_buffer_handle));
    }

    Ok(vec)
}

pub fn js_array_of_buffers_to_identifiers<'a, C: Context<'a>>(
    cx: &mut C,
    js_array: Handle<JsArray>,
) -> NeonResult<Vec<[u8; 32]>> {
    let buf_vec = js_array.to_vec(cx)?;
    let mut vec: Vec<[u8; 32]> = Vec::with_capacity(buf_vec.len());

    for buf in buf_vec {
        let js_buffer_handle = buf.downcast_or_throw::<JsBuffer, _>(cx)?;
        vec.push(js_buffer_to_identifier(cx, js_buffer_handle)?);
    }

    Ok(vec)
}

pub fn js_value_to_option<'a, T: Value, C: Context<'a>>(
    cx: &mut C,
    js_value: Handle<'a, JsValue>,
) -> NeonResult<Option<Handle<'a, T>>> {
    if js_value.is_a::<JsNull, _>(cx) || js_value.is_a::<JsUndefined, _>(cx) {
        Ok(None)
    } else {
        Ok(Some(js_value.downcast_or_throw::<T, _>(cx)?))
    }
}

fn js_object_get_vec_u8<'a, C: Context<'a>>(
    cx: &mut C,
    js_object: Handle<JsObject>,
    field: &str,
) -> NeonResult<Vec<u8>> {
    let buffer: Handle<JsBuffer> = js_object.get(cx, field)?;

    Ok(js_buffer_to_vec_u8(cx, buffer))
}

fn js_object_to_query<'a, C: Context<'a>>(
    cx: &mut C,
    js_object: Handle<JsObject>,
) -> NeonResult<Query> {
    let items: Handle<JsArray> = js_object.get(cx, "items")?;
    let mut query = Query::new();
    for js_item in items.to_vec(cx)? {
        let item = js_item.downcast_or_throw::<JsObject, _>(cx)?;

        let item_type: Handle<JsString> = item.get(cx, "type")?;
        let item_type = item_type.value(cx);

        match item_type.as_ref() {
            "key" => {
                query.insert_key(js_object_get_vec_u8(cx, item, "key")?);
            }
            "range" => {
                let from = js_object_get_vec_u8(cx, item, "from")?;
                let to = js_object_get_vec_u8(cx, item, "to")?;
                query.insert_range(from..to);
            }
            "rangeInclusive" => {
                let from = js_object_get_vec_u8(cx, item, "from")?;
                let to = js_object_get_vec_u8(cx, item, "to")?;
                query.insert_range_inclusive(from..=to);
            }
            "rangeFull" => {
                query.insert_all();
            }
            "rangeFrom" => {
                query.insert_range_from(js_object_get_vec_u8(cx, item, "from")?..);
            }
            "rangeTo" => {
                query.insert_range_to(..js_object_get_vec_u8(cx, item, "to")?);
            }
            "rangeToInclusive" => {
                query.insert_range_to_inclusive(..=js_object_get_vec_u8(cx, item, "to")?);
            }
            "rangeAfter" => {
                query.insert_range_after(js_object_get_vec_u8(cx, item, "after")?..);
            }
            "rangeAfterTo" => {
                let after = js_object_get_vec_u8(cx, item, "after")?;
                let to = js_object_get_vec_u8(cx, item, "to")?;
                query.insert_range_after_to(after..to);
            }
            "rangeAfterToInclusive" => {
                let after = js_object_get_vec_u8(cx, item, "after")?;
                let to = js_object_get_vec_u8(cx, item, "to")?;
                query.insert_range_after_to_inclusive(after..=to);
            }
            _ => {
                cx.throw_range_error("query item type is not supported")?;
            }
        }
    }

    let subquery_path = js_value_to_option::<JsArray, _>(cx, js_object.get(cx, "subqueryPath")?)?
        .map(|x| js_array_of_buffers_to_vec(cx, x))
        .transpose()?;
    let subquery = js_value_to_option::<JsObject, _>(cx, js_object.get(cx, "subquery")?)?
        .map(|x| js_object_to_query(cx, x))
        .transpose()?;
    let left_to_right = js_value_to_option::<JsBoolean, _>(cx, js_object.get(cx, "leftToRight")?)?
        .map(|x| x.value(cx));

    query.default_subquery_branch.subquery_path = subquery_path;
    query.default_subquery_branch.subquery = subquery.map(Box::new);
    query.left_to_right = left_to_right.unwrap_or(true);

    Ok(query)
}

fn js_object_to_sized_query<'a, C: Context<'a>>(
    cx: &mut C,
    js_object: Handle<JsObject>,
) -> NeonResult<SizedQuery> {
    let query: Handle<JsObject> = js_object.get(cx, "query")?;
    let query = js_object_to_query(cx, query)?;

    let limit: Option<u16> = js_value_to_option::<JsNumber, _>(cx, js_object.get(cx, "limit")?)?
        .map(|x| {
            u16::try_from(x.value(cx) as i64)
                .or_else(|_| cx.throw_range_error("`limit` must fit in u16"))
        })
        .transpose()?;
    let offset: Option<u16> = js_value_to_option::<JsNumber, _>(cx, js_object.get(cx, "offset")?)?
        .map(|x| {
            u16::try_from(x.value(cx) as i64)
                .or_else(|_| cx.throw_range_error("`offset` must fit in u16"))
        })
        .transpose()?;

    Ok(SizedQuery::new(query, limit, offset))
}

pub fn js_path_query_to_path_query<'a, C: Context<'a>>(
    cx: &mut C,
    js_path_query: Handle<JsObject>,
) -> NeonResult<PathQuery> {
    let path = js_array_of_buffers_to_vec(cx, js_path_query.get(cx, "path")?)?;
    let query = js_object_to_sized_query(cx, js_path_query.get(cx, "query")?)?;

    Ok(PathQuery::new(path, query))
}

pub fn js_object_to_block_info<'a, C: Context<'a>>(
    cx: &mut C,
    js_object: Handle<JsObject>,
) -> NeonResult<BlockInfo> {
    let js_height: Handle<JsNumber> = js_object.get(cx, "height")?;
    let js_epoch: Handle<JsNumber> = js_object.get(cx, "epoch")?;
    let js_time: Handle<JsNumber> = js_object.get(cx, "timeMs")?;

    let epoch = Epoch::new(js_epoch.value(cx) as u16);

    let block_info = BlockInfo {
        height: js_height.value(cx) as u64,
        time_ms: js_time.value(cx) as u64,
        epoch,
    };

    Ok(block_info)
}

pub fn js_object_to_identity_public_key<'a, C: Context<'a>>(
    cx: &mut C,
    js_object: Handle<JsObject>,
) -> NeonResult<IdentityPublicKey> {
    let js_id: Handle<JsNumber> = js_object.get(cx, "id")?;
    let js_purpose: Handle<JsNumber> = js_object.get(cx, "purpose")?;
    let js_security_level: Handle<JsNumber> = js_object.get(cx, "securityLevel")?;
    let js_key_type: Handle<JsNumber> = js_object.get(cx, "type")?;
    let js_read_only: Handle<JsBoolean> = js_object.get(cx, "readOnly")?;
    let js_data: Handle<JsBuffer> = js_object.get(cx, "data")?;
    let js_disabled_at: Handle<JsValue> = js_object.get(cx, "disabledAt")?;

    let id = js_id.value(cx) as KeyID;

    let purpose = Purpose::try_from(js_purpose.value(cx) as u8)
        .or_else(|_| cx.throw_range_error("`purpose` value is incorrect"))?;

    let security_level = SecurityLevel::try_from(js_security_level.value(cx) as u8)
        .or_else(|_| cx.throw_range_error("`securityLevel` value is incorrect"))?;

    let key_type = KeyType::try_from(js_key_type.value(cx) as u8)
        .or_else(|_| cx.throw_range_error("`keyType` value is incorrect"))?;

    let read_only = js_read_only.value(cx);

    let data = js_buffer_to_vec_u8(cx, js_data);

    let disabled_at: Option<u64> = js_value_to_option::<JsNumber, _>(cx, js_disabled_at)?
        .map(|x| {
            u64::try_from(x.value(cx) as i64)
                .or_else(|_| cx.throw_range_error("`offset` must fit in u16"))
        })
        .transpose()?;

    Ok(IdentityPublicKey {
        id,
        purpose,
        security_level,
        key_type,
        read_only,
        data,
        disabled_at,
    })
}

pub fn js_array_to_keys<'a, C: Context<'a>>(
    cx: &mut C,
    js_array: Handle<JsArray>,
) -> NeonResult<Vec<IdentityPublicKey>> {
    let keys = js_array
        .to_vec(cx)?
        .into_iter()
        .map(|js_value| {
            let js_key = js_value.downcast_or_throw::<JsObject, _>(cx)?;
            let key = js_object_to_identity_public_key(cx, js_key)?;

            Ok(key)
        })
        .collect::<Result<_, _>>()?;

    Ok(keys)
}

pub fn js_object_to_fee_refunds<'a, C: Context<'a>>(
    cx: &mut C,
    js_object: Handle<JsObject>,
) -> NeonResult<CreditsPerEpoch> {
    let mut fee_refunds: CreditsPerEpoch = Default::default();

    for js_epoch_index_value in js_object.get_own_property_names(cx)?.to_vec(cx)? {
        let js_epoch_index = js_epoch_index_value.downcast_or_throw::<JsString, _>(cx)?;

        let epoch_index = js_epoch_index
            .value(cx)
            .parse()
            .or_else(|e: ParseIntError| cx.throw_error(e.to_string()))?;

        let js_credits: Handle<JsNumber> = js_object.get(cx, js_epoch_index)?;
        let credits = js_credits.value(cx) as Credits;

        fee_refunds.insert(epoch_index, credits);
    }

    Ok(fee_refunds)
}
