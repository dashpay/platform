const { default: Ajv } = require('ajv');

const addFormats = require('ajv-formats');
const addByteArrayKeyword = require('./keywords/byteArray/addByteArrayKeyword');

/**
 * @return {ajv.Ajv}
 */
function createAjv() {
  const ajv = new Ajv({
    strictTypes: true,
    strictTuples: true,
    strictRequired: true,
    addUsedSchema: false,
    strict: true,
  });

  addFormats(ajv, { mode: 'fast' });

  addByteArrayKeyword(ajv);

  return ajv;
}

module.exports = createAjv;
