// eslint-disable-next-line max-len
const castStorageItemsTypes = (originalItem, schema) => {
  if (!schema) {
    throw new Error('Schema is undefined');
  }

  return Object.entries(schema).reduce((acc, next) => {
    const [schemaKey, schemaValue] = next;

    const item = originalItem[schemaKey];
    const result = {};

    if (schemaKey !== '*' && item === undefined) {
      throw new Error(`No item found for schema key "${schemaKey}" in item ${JSON.stringify(originalItem)}`);
    }

    if (schemaValue.constructor.name !== 'Object') {
      const Clazz = schemaValue;

      if (schemaKey === '*') {
        Object.keys(originalItem).forEach((itemKey) => {
          result[itemKey] = new Clazz(originalItem[itemKey]);
        });
      } else {
        // eslint-disable-next-line valid-typeof
        if (typeof schemaValue === 'string' && typeof item !== schemaValue) {
          throw new Error(`Invalid schema type for key "${schemaKey}" in item ${JSON.stringify(originalItem)}`);
        }

        result[schemaKey] = typeof schemaValue === 'string' ? item : new Clazz(item);
      }
    } else if (schemaKey === '*') {
      Object
        .entries(originalItem)
        .forEach(([key, value]) => {
          result[key] = castStorageItemsTypes(value, schemaValue);
        }, {});
    } else {
      result[schemaKey] = castStorageItemsTypes(item, schemaValue);
    }

    return { ...acc, ...result };
  }, {});
};
module.exports = castStorageItemsTypes;
