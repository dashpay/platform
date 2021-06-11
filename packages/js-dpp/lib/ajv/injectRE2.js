const { default: getRE2Class } = require('@dashevo/re2-wasm');

const codegen = require('ajv/dist/compile/codegen');
const code = require('ajv/dist/vocabularies/code');

async function injectRE2() {
  const RE2 = await getRE2Class();

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
