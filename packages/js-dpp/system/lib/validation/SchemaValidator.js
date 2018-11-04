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
   * @param {object} schema
   * @param {object} object
   * @param {array|Object} additionalSchemas
   * @return {array}
   */
  validate(schema, object, additionalSchemas = {}) {
    for (const schemaId of Object.keys(additionalSchemas)) {
      this.ajv.addSchema(additionalSchemas[schemaId], schemaId);
    }

    // TODO: Use { schemas: additionalSchemas }
    this.ajv.validate(schema, object);

    for (const schemaId of Object.keys(additionalSchemas)) {
      this.ajv.removeSchema(schemaId);
    }

    if (this.ajv.errors) {
      return this.ajv.errors;
    }

    return [];
  }
}

SchemaValidator.SCHEMAS = {
  META: {
    DAP_CONTRACT: 'https://schema.dash.org/platform-4-0-0/system/meta/dap-contract',
  },
  BASE: {
    DAP_OBJECT: 'https://schema.dash.org/platform-4-0-0/system/base/dap-object',
  },
  ST_PACKET: 'https://schema.dash.org/platform-4-0-0/system/st-packet',
};

module.exports = SchemaValidator;
