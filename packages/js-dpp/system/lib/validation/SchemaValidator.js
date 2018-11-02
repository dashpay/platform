const dashSchema = require('../../../dash-schema/schema/schema');
const dapObjectBaseSchema = require('../../schema/base/dap-object');
const dapContractMetaSchema = require('../../schema/meta/dap-contract');
const stPacketSchema = require('../../schema/st-packet');

class SchemaValidator {
  constructor(ajv) {
    this.ajv = ajv;

    this.ajv.addMetaSchema(dashSchema);

    this.ajv.addSchema(dapObjectBaseSchema);
    this.ajv.addSchema(stPacketSchema);

    this.ajv.addMetaSchema(dapContractMetaSchema);
  }

  /**
   *
   * @param {DapContract} dapContract
   */
  setDapContract(dapContract) {
    // TODO handle dap contract errors
    this.ajv.addSchema(dapContract.toJSON(), SchemaValidator.SHEMAS.DAP_CONTRACT);
  }

  /**
   * @param {object} schema
   * @param {object} object
   * @return {array}
   */
  validate(schema, object) {
    this.ajv.validate(schema, object);

    if (this.ajv.errors) {
      return this.ajv.errors;
    }

    return [];
  }
}

SchemaValidator.SHEMAS = {
  META: {
    DAP_CONTRACT: 'https://schema.dash.org/platform-4-0-0/system/meta/dap-contract',
  },
  BASE: {
    DAP_OBJECT: 'https://schema.dash.org/platform-4-0-0/system/base/dap-object',
  },
  ST_PACKET: 'https://schema.dash.org/platform-4-0-0/system/st-packet',
  DAP_CONTRACT: 'dap-contract',
};

module.exports = SchemaValidator;
