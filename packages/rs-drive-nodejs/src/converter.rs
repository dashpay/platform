use grovedb::{Element, PathQuery, Query, SizedQuery};
use neon::{borrow::Borrow, prelude::*};

fn element_to_string(element: Element) -> String {
    match element {
        Element::Item(_) => "item".to_string(),
        Element::Reference(_) => "reference".to_string(),
        Element::Tree(_) => "tree".to_string(),
    }
}

pub fn js_object_to_element<'a, C: Context<'a>>(
    js_object: Handle<JsObject>,
    cx: &mut C,
) -> NeonResult<Element> {
    let js_element_string = js_object.get(cx, "type")?.to_string(cx)?;
    let value = js_object.get(cx, "value")?;

    let element_string: String = js_element_string.value(cx);

    match element_string.as_str() {
        "item" => {
            let js_buffer = value.downcast_or_throw::<JsBuffer, _>(cx)?;
            let item = js_buffer_to_vec_u8(js_buffer, cx);
            Ok(Element::Item(item))
        }
        "reference" => {
            let js_array = value.downcast_or_throw::<JsArray, _>(cx)?;
            let reference = js_array_of_buffers_to_vec(js_array, cx)?;
            Ok(Element::Reference(reference))
        }
        "tree" => {
            let js_buffer = value.downcast_or_throw::<JsBuffer, _>(cx)?;
            let tree_vec = js_buffer_to_vec_u8(js_buffer, cx);
            Ok(Element::Tree(tree_vec.try_into().or_else(
                |v: Vec<u8>| {
                    cx.throw_error(format!(
                        "Tree buffer is expected to be 32 bytes long, but got {}",
                        v.len()
                    ))
                },
            )?))
        }
        _ => cx.throw_error(format!("Unexpected element type {}", element_string)),
    }
}

pub fn element_to_js_object<'a, C: Context<'a>>(
    element: Element,
    cx: &mut C,
) -> NeonResult<Handle<'a, JsValue>> {
    let js_object = cx.empty_object();
    let js_type_string = cx.string(element_to_string(element.clone()));
    js_object.set(cx, "type", js_type_string)?;

    let js_value: Handle<JsValue> = match element {
        Element::Item(item) => {
            let js_buffer = JsBuffer::external(cx, item);
            js_buffer.upcast()
        }
        Element::Reference(reference) => nested_vecs_to_js(reference, cx)?,
        Element::Tree(tree) => {
            let js_buffer = JsBuffer::external(cx, tree);
            js_buffer.upcast()
        }
    };

    js_object.set(cx, "value", js_value)?;
    NeonResult::Ok(js_object.upcast())
}

pub fn nested_vecs_to_js<'a, C: Context<'a>>(
    v: Vec<Vec<u8>>,
    cx: &mut C,
) -> NeonResult<Handle<'a, JsValue>> {
    let js_array: Handle<JsArray> = cx.empty_array();

    for (index, bytes) in v.iter().enumerate() {
        let js_buffer = JsBuffer::external(cx, bytes.clone());
        let js_value = js_buffer.as_value(cx);
        js_array.set(cx, index as u32, js_value)?;
    }

    Ok(js_array.upcast())
}

pub fn js_buffer_to_vec_u8<'a, C: Context<'a>>(js_buffer: Handle<JsBuffer>, cx: &mut C) -> Vec<u8> {
    let guard = cx.lock();
    // let key_buffer = js_buffer.deref();
    let key_memory_view = js_buffer.borrow(&guard);
    let key_slice = key_memory_view.as_slice::<u8>();
    key_slice.to_vec()
}

pub fn js_array_of_buffers_to_vec<'a, C: Context<'a>>(
    js_array: Handle<JsArray>,
    cx: &mut C,
) -> NeonResult<Vec<Vec<u8>>> {
    let buf_vec = js_array.to_vec(cx)?;
    let mut vec: Vec<Vec<u8>> = Vec::new();

    for buf in buf_vec {
        let js_buffer_handle = buf.downcast_or_throw::<JsBuffer, _>(cx)?;
        vec.push(js_buffer_to_vec_u8(js_buffer_handle, cx));
    }

    Ok(vec)
}

pub fn js_value_to_option<'a, T: Value, C: Context<'a>>(
    js_value: Handle<'a, JsValue>,
    cx: &mut C,
) -> NeonResult<Option<Handle<'a, T>>> {
    if js_value.is_a::<JsNull, _>(cx) || js_value.is_a::<JsUndefined, _>(cx) {
        Ok(None)
    } else {
        Ok(Some(js_value.downcast_or_throw::<T, _>(cx)?))
    }
}

fn js_object_get_vec_u8<'a, C: Context<'a>>(
    js_object: Handle<JsObject>,
    field: &str,
    cx: &mut C,
) -> NeonResult<Vec<u8>> {
    Ok(js_buffer_to_vec_u8(
        js_object
            .get(cx, field)?
            .downcast_or_throw::<JsBuffer, _>(cx)?,
        cx,
    ))
}

fn js_object_to_query<'a, C: Context<'a>>(
    js_object: Handle<JsObject>,
    cx: &mut C,
) -> NeonResult<Query> {
    let items = js_object
        .get(cx, "items")?
        .downcast_or_throw::<JsArray, _>(cx)?;
    let mut query = Query::new();
    for js_item in items.to_vec(cx)? {
        let item = js_item.downcast_or_throw::<JsObject, _>(cx)?;
        match item
            .get(cx, "type")?
            .downcast_or_throw::<JsString, _>(cx)?
            .value(cx)
            .as_ref()
        {
            "key" => {
                query.insert_key(js_object_get_vec_u8(item, "key", cx)?);
            }
            "range" => {
                let from = js_object_get_vec_u8(item, "from", cx)?;
                let to = js_object_get_vec_u8(item, "to", cx)?;
                query.insert_range(from..to);
            }
            "rangeInclusive" => {
                let from = js_object_get_vec_u8(item, "from", cx)?;
                let to = js_object_get_vec_u8(item, "to", cx)?;
                query.insert_range_inclusive(from..=to);
            }
            "rangeFull" => {
                query.insert_all();
            }
            "rangeFrom" => {
                query.insert_range_from(js_object_get_vec_u8(item, "from", cx)?..);
            }
            "rangeTo" => {
                query.insert_range_to(..js_object_get_vec_u8(item, "to", cx)?);
            }
            "rangeToInclusive" => {
                query.insert_range_to_inclusive(..=js_object_get_vec_u8(item, "to", cx)?);
            }
            "rangeAfter" => {
                query.insert_range_after(js_object_get_vec_u8(item, "after", cx)?..);
            }
            "rangeAfterTo" => {
                let after = js_object_get_vec_u8(item, "after", cx)?;
                let to = js_object_get_vec_u8(item, "to", cx)?;
                query.insert_range_after_to(after..to);
            }
            "rangeAfterToInclusive" => {
                let after = js_object_get_vec_u8(item, "after", cx)?;
                let to = js_object_get_vec_u8(item, "to", cx)?;
                query.insert_range_after_to_inclusive(after..=to);
            }
            _ => {
                cx.throw_range_error("query item type is not supported")?;
            }
        }
    }

    let subquery_key = js_value_to_option::<JsBuffer, _>(js_object.get(cx, "subqueryKey")?, cx)?
        .map(|x| js_buffer_to_vec_u8(x, cx));
    let subquery = js_value_to_option::<JsObject, _>(js_object.get(cx, "subquery")?, cx)?
        .map(|x| js_object_to_query(x, cx))
        .transpose()?;
    let left_to_right = js_value_to_option::<JsBoolean, _>(js_object.get(cx, "leftToRight")?, cx)?
        .map(|x| x.value(cx));

    query.subquery_key = subquery_key;
    query.subquery = subquery.map(Box::new);
    query.left_to_right = left_to_right.unwrap_or(true);

    Ok(query)
}

fn js_object_to_sized_query<'a, C: Context<'a>>(
    js_object: Handle<JsObject>,
    cx: &mut C,
) -> NeonResult<SizedQuery> {
    let query = js_object_to_query(
        js_object
            .get(cx, "query")?
            .downcast_or_throw::<JsObject, _>(cx)?,
        cx,
    )?;
    let limit: Option<u16> = js_value_to_option::<JsNumber, _>(js_object.get(cx, "limit")?, cx)?
        .map(|x| {
            u16::try_from(x.value(cx) as i64)
                .or_else(|_| cx.throw_range_error("`limit` must fit in u16"))
        })
        .transpose()?;
    let offset: Option<u16> = js_value_to_option::<JsNumber, _>(js_object.get(cx, "offset")?, cx)?
        .map(|x| {
            u16::try_from(x.value(cx) as i64)
                .or_else(|_| cx.throw_range_error("`offset` must fit in u16"))
        })
        .transpose()?;
    Ok(SizedQuery::new(query, limit, offset))
}

pub fn js_path_query_to_path_query<'a, C: Context<'a>>(
    js_path_query: Handle<JsObject>,
    cx: &mut C,
) -> NeonResult<PathQuery> {
    let path =
        js_array_of_buffers_to_vec(js_path_query.get(cx, "path")?.downcast_or_throw(cx)?, cx)?;
    let query =
        js_object_to_sized_query(js_path_query.get(cx, "query")?.downcast_or_throw(cx)?, cx)?;
    Ok(PathQuery::new(path, query))
}
