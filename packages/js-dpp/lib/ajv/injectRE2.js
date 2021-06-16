const codegen = require('ajv/dist/compile/codegen');
const code = require('ajv/dist/vocabularies/code');

/**
 * @param {Function} RE2
 */
function injectRE2(RE2) {
  global.RE2 = RE2;

  code.usePattern = function usePattern({ gen }, pattern) {
    return gen.scopeValue('pattern', {
      key: pattern,
      ref: new RE2(pattern, 'u'),
      code: codegen._`new RE2(${pattern}, "u")`,
    });
  };
}

module.exports = injectRE2;
