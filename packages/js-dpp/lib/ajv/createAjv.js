const { default: Ajv } = require('ajv/dist/2020');

const addFormats = require('ajv-formats');

const addByteArrayKeyword = require('./keywords/byteArray/addByteArrayKeyword');

const injectRE2 = require('./injectRE2');

/**
 * @param {Function} RE2
 * @return {Ajv2020}
 */
function createAjv(RE2) {
  injectRE2(RE2);

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
