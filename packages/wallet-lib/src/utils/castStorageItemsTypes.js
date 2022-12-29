const createCastFunction = (schemaValue) => {
  let castFunction;
  if (typeof schemaValue === 'string') {
    castFunction = (itemToCast) => {
      // eslint-disable-next-line valid-typeof
      if (typeof itemToCast !== schemaValue) {
        throw new Error(`Value "${itemToCast}" is not of type "${schemaValue}"`);
      }
      return itemToCast;
    };
  } else if (typeof schemaValue === 'function') {
    castFunction = schemaValue;
  } else {
    castFunction = (itemToCast) => {
      const Clazz = schemaValue;
      return new Clazz(itemToCast);
    };
  }

  return castFunction;
};

/**
 * Performs casting of items to specified schema.
 * Supports nested objects, but does not support nested arrays or arrays of objects.
 * @param originalItem
 * @param schema
 * @returns {{}|*}
 */
const castStorageItemsTypes = (originalItem, schema, path = '') => {
  if (!schema) {
    throw new Error('Schema is undefined');
  }

  if (Array.isArray(schema)) {
    const schemaValue = schema[0];

    const schemaType = schemaValue.constructor.name;
    if (schemaType !== 'Object' && schemaType !== 'Array') {
      const castItem = createCastFunction(schemaValue);
      return originalItem.map((element) => castItem(element));
    }
    throw new Error('Casting of nested arrays and arrays of objects is not supported.');
  }

  return Object.entries(schema).reduce((acc, next) => {
    const [schemaKey, schemaValue] = next;
    const result = {};

    if (schemaKey !== '*' && originalItem[schemaKey] === undefined) {
      throw new Error(`No item found for schema key "${schemaKey}" in path "${path}"`);
    }

    const schemaType = schemaValue.constructor.name;
    if (schemaType !== 'Object' && schemaType !== 'Array') {
      const castItem = createCastFunction(schemaValue);

      if (schemaKey === '*') {
        Object.keys(originalItem).forEach((itemKey) => {
          result[itemKey] = castItem(originalItem[itemKey]);
        });
      } else {
        result[schemaKey] = castItem(originalItem[schemaKey]);
      }
    } else if (schemaKey === '*') {
      Object
        .entries(originalItem)
        .forEach(([key, value]) => {
          result[key] = castStorageItemsTypes(value, schemaValue, `${path}.${key}`);
        }, {});
    } else {
      result[schemaKey] = castStorageItemsTypes(originalItem[schemaKey], schemaValue, `${path}.${schemaKey}`);
    }

    return { ...acc, ...result };
  }, {});
};
module.exports = castStorageItemsTypes;
