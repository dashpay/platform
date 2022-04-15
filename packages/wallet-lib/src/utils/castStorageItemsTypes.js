// eslint-disable-next-line max-len
const castStorageItemsTypes = (originalItem, schema) => Object.entries(schema).reduce((acc, next) => {
  const [schemaKey, schemaValue] = next;

  const item = originalItem[schemaKey];
  const result = {};

  if (schemaKey !== '*' && item === undefined) {
    throw new Error(`No schema found for key "${schemaKey}" in item ${JSON.stringify(originalItem)}`);
  }

  if (schemaValue.constructor.name !== 'Object') {
    const Clazz = schemaValue;
    if (schemaKey === '*') {
      Object.keys(originalItem).forEach((itemKey) => {
        result[itemKey] = new Clazz(originalItem[itemKey]);
      });
    } else {
      // eslint-disable-next-line no-lonely-if
      if (typeof schemaValue === 'string') {
        // eslint-disable-next-line valid-typeof
        if (typeof item !== schemaValue) {
          throw new Error(`Invalid schema type for key "${schemaKey}" in item ${JSON.stringify(originalItem)}`);
        }

        result[schemaKey] = item;
      } else {
        result[schemaKey] = new Clazz(item);
      }
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

module.exports = castStorageItemsTypes;
