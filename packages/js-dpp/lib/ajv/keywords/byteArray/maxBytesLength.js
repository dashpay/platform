/**
 * @param {number} schema
 * @param {Buffer} data
 * @return {boolean}
 */
function validate(schema, data) {
  if (data.length > schema) {
    validate.errors = [{
      keyword: 'maxBytesLength',
      message: `should NOT be longer than ${schema} bytes`,
      params: {
        limit: schema,
      },
    }];

    return false;
  }

  return true;
}

const maxBytesLengthKeyword = {
  type: 'object',
  dependencies: ['byteArray'],
  metaSchema: {
    type: 'integer',
    minimum: 0,
  },
  validate,
};

module.exports = maxBytesLengthKeyword;
