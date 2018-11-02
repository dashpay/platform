const Ajv = require('ajv');

const dashSchema = require('../../../dash-schema/schema/schema');
const dapObjectBaseSchema = require('../../schema/base/dap-object');
const appContractMetaSchema = require('../../schema/meta/dap-contract');

module.exports = function validateDapContract(dapContract) {
  const ajv = new Ajv();

  ajv.addMetaSchema(dashSchema);

  ajv.addSchema(dapObjectBaseSchema);

  // TODO: Use validateSchema?

  ajv.validate(appContractMetaSchema, dapContract);

  if (ajv.errors) {
    return ajv.errors;
  }

  return null;
};
