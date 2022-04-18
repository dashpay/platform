const castStorageItemsTypes = (originalItem, schema) => {
  if (!schema) {
    throw new Error('Schema is undefined');
  }

  return Object.entries(schema).reduce((acc, next) => {
    const [schemaKey, schemaValue] = next;
    const result = {};

    if (schemaKey !== '*' && originalItem[schemaKey] === undefined) {
      throw new Error(`No item found for schema key "${schemaKey}" in item ${JSON.stringify(originalItem)}`);
    }

    if (schemaValue.constructor.name !== 'Object') {
      let castItem;

      if (typeof schemaValue === 'string') {
        castItem = (itemToCast) => {
          // eslint-disable-next-line valid-typeof
          if (typeof itemToCast !== schemaValue) {
            throw new Error(`Value "${itemToCast}" is not of type "${schemaValue}"`);
          }
          return itemToCast;
        };
      } else if (typeof schemaValue === 'function') {
        castItem = schemaValue;
      } else {
        castItem = (itemToCast) => {
          const Clazz = schemaValue;
          return new Clazz(itemToCast);
        };
      }

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
          result[key] = castStorageItemsTypes(value, schemaValue);
        }, {});
    } else {
      result[schemaKey] = castStorageItemsTypes(originalItem[schemaKey], schemaValue);
    }

    return { ...acc, ...result };
  }, {});
};
module.exports = castStorageItemsTypes;
