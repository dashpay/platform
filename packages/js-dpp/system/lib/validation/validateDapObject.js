const Ajv = require('ajv');

const dashSchema = require('../../../dash-schema/schema/schema');
const dapObjectBaseSchema = require('../../schema/base/dap-object');
const dapContractMetaSchema = require('../../schema/meta/dap-contract');

module.exports = function validateDapObject(object, dapContract) {
  const ajv = new Ajv();

  ajv.addMetaSchema(dashSchema);

  ajv.addSchema(dapObjectBaseSchema);

  ajv.addMetaSchema(dapContractMetaSchema);

  ajv.addSchema(dapContract, 'dap-contract');

  ajv.validate('https://schema.dash.org/platform-4-0-0/system/base/dap-object', object);

  if (ajv.errors) {
    return ajv.errors;
  }

  try {
    ajv.validate({
      $ref: `dap-contract#/objectsDefinition/${object.$$type}`,
    }, object);
  } catch (e) {
    if (e.missingSchema === 'dap-contract') {
      return [e];
    }

    throw e;
  }

  if (ajv.errors) {
    return ajv.errors;
  }

  return null;
};
