/**
 * @param {boolean} schema
 * @param {*} data
 * @return {boolean}
 */
function validate(schema, data) {
  if (!Buffer.isBuffer(data)) {
    validate.errors = [{
      keyword: 'byteArray',
      message: 'should be a byte array',
    }];

    return false;
  }

  return true;
}

const byteArray = {
  type: 'object',
  metaSchema: {
    type: 'boolean',
    const: true,
  },
  validate,
};

module.exports = byteArray;
