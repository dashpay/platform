const { default: Ajv } = require('ajv');

const addFormats = require('ajv-formats');

const addByteArrayKeyword = require('./keywords/byteArray/addByteArrayKeyword');

const injectRE2 = require('./injectRE2');

/**
 * @return {Promise<ajv.Ajv>}
 */
async function createAjv() {
  await injectRE2();

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
