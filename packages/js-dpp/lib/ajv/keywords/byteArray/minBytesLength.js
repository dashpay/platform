/**
 * @param {number} schema
 * @param {Buffer} data
 * @return {boolean}
 */
function validate(schema, data) {
  if (data.length < schema) {
    validate.errors = [{
      keyword: 'minBytesLength',
      message: `should NOT be shorter than ${schema} bytes`,
      params: {
        limit: schema,
      },
    }];

    return false;
  }

  return true;
}

const minBytesLengthKeyword = {
  type: 'object',
  dependencies: ['byteArray'],
  metaSchema: {
    type: 'integer',
    minimum: 0,
  },
  validate,
};

module.exports = minBytesLengthKeyword;
