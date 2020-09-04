/**
 * Recursively build properties map
 *
 * @param {Object} schema
 * @param {string} [propertyName=undefined]
 *
 * @return {Object}
 */
function buildEncodedPropertiesMap(schema, propertyName = undefined) {
  const propertyNames = Object.keys(schema.properties);

  // We iterate over every property defined in the document schema while
  // building a flat map e.g.
  //
  // {
  //   "firstLevel.secondLevel": { ...property keywords },
  //   "firstLevel.secondLevel.third[0].property": { ...property keywords },
  // }
  //
  // of every property that have `contentEncoding` keyword
  return propertyNames.reduce((map, name) => {
    const property = schema.properties[name];

    const propertyPath = propertyName ? `${propertyName}.${name}` : name;

    if (property.type === 'object') {
      // In case property is an object we recursively call build method
      // passing property as schema and assigning property path to current property name,
      // this will allow for property name chaining e.g. `first.second: value`
      // then we flatten the result and add to our resulting map

      // eslint-disable-next-line no-param-reassign
      map = {
        ...map,
        ...buildEncodedPropertiesMap(property, propertyPath),
      };
    }

    if (property.type === 'array' && property.items.type === 'object') {
      // In case property is an array of a single type we recursively call build method
      // passing array `item` property as schema and assigning property path to current
      // property name, this will allow for property name chaining e.g. `first.second: value`
      // then we flatten the result and add to our resulting map

      // eslint-disable-next-line no-param-reassign
      map = {
        ...map,
        ...buildEncodedPropertiesMap(property.items, propertyPath),
      };
    }

    if (property.type === 'array' && Array.isArray(property.items)) {
      // In case property is an array of arrays
      // We build a schema by assigning every item in the array schema to an index property name e.g
      //
      // {
      //   "arrayPropertyName[0]": { ...item 0 object },
      //   "arrayPropertyName[1]": { ...item 1 object },
      //   ...
      // }
      //
      // and recursively call build method passing resulting schema and empty property name
      // to avoid duplication of the property name in the resulting object,
      // this will allow for property name chaining e.g. `first.second: value`
      // then we flatten the result and add to our resulting map

      const arraySchema = property.items.reduce((schemaObject, item, index) => {
        // eslint-disable-next-line no-param-reassign
        schemaObject.properties[`${propertyPath}[${index}]`] = item;

        return schemaObject;
      }, {
        properties: {},
      });

      // eslint-disable-next-line no-param-reassign
      map = {
        ...map,
        ...buildEncodedPropertiesMap(arraySchema),
      };
    }

    if (property.contentEncoding !== undefined) {
      // eslint-disable-next-line no-param-reassign
      map[propertyPath] = property;
    }

    return map;
  }, {});
}

/**
 * Construct and get all properties with `contentEncoding` keyword
 *
 * @param {Object} documentSchema
 *
 * @return {Object}
 */
function getEncodedPropertiesFromSchema(documentSchema) {
  if (!documentSchema.properties) {
    return {};
  }

  return buildEncodedPropertiesMap(documentSchema);
}

module.exports = getEncodedPropertiesFromSchema;
