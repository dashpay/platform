const SCHEMA = {
  lastKnownBlock: {
    height: 'number',
  },
};

const castItemTypes = (item, schema) => {
  console.log('asdasd')
  Object.entries(schema).forEach(([schemaKey, schemaValue]) => {
    if (schemaValue.constructor.name !== 'Object') {
      const Clazz = schemaValue;
      if (schemaKey === '*') {
        Object.keys(item).forEach((itemKey) => {
          // eslint-disable-next-line no-param-reassign
          item[itemKey] = new Clazz(item[itemKey]);
        });
      } else {
        if (!item[schemaKey]) {
          throw new Error(`No schema key "${schemaKey}" found for item ${JSON.stringify(item)}`);
        }

        // todo typeof
        if (!(['string', 'number'].includes(schemaValue))) {
          // eslint-disable-next-line no-param-reassign
          item[schemaKey] = new Clazz(item[schemaKey]);
        }
      }
    } else if (schemaKey === '*') {
      Object.values(item).forEach((itemValue) => castItemTypes(itemValue, schemaValue));
    } else {
      castItemTypes(item[schemaKey], schemaValue);
    }
  });

  return item;
};

function importState(state) {
  try {
    castItemTypes(state, SCHEMA)
  } catch (e) {
    console.error(e)
  }

  this.state.lastKnownBlock = state.lastKnownBlock;
}

module.exports = importState;

