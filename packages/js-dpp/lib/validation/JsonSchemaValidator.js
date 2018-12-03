const dashSchema = require('../../schema/meta/dash-schema');
const dapObjectBaseSchema = require('../../schema/base/dap-object');
const dapContractMetaSchema = require('../../schema/meta/dap-contract');
const stPacketSchema = require('../../schema/st-packet');
const stPacketHeaderSchema = require('../../schema/st-packet-header');

const ValidationResult = require('./ValidationResult');
const JsonSchemaError = require('../consensusErrors/JsonSchemaError');

class JsonSchemaValidator {
  constructor(ajv) {
    this.ajv = ajv;

    this.ajv.addMetaSchema(dashSchema);

    this.ajv.addSchema(dapObjectBaseSchema);
    this.ajv.addSchema(stPacketSchema);
    this.ajv.addSchema(stPacketHeaderSchema);

    this.ajv.addMetaSchema(dapContractMetaSchema);
  }

  /**
   * @param {object} schema
   * @param {object} object
   * @param {array|Object} additionalSchemas
   * @return {ValidationResult}
   */
  validate(schema, object, additionalSchemas = {}) {
    // TODO Keep cached/compiled additional schemas

    Object.keys(additionalSchemas).forEach((schemaId) => {
      this.ajv.addSchema(additionalSchemas[schemaId], schemaId);
    });

    this.ajv.validate(schema, object);

    Object.keys(additionalSchemas).forEach((schemaId) => {
      this.ajv.removeSchema(schemaId);
    });

    return new ValidationResult(
      (this.ajv.errors || []).map(error => new JsonSchemaError(error)),
    );
  }
}

JsonSchemaValidator.SCHEMAS = {
  META: {
    DAP_CONTRACT: 'https://schema.dash.org/platform-4-0-0/system/meta/dap-contract',
  },
  BASE: {
    DAP_OBJECT: 'https://schema.dash.org/platform-4-0-0/system/base/dap-object',
  },
  ST_PACKET: 'https://schema.dash.org/platform-4-0-0/system/st-packet',
  ST_PACKET_HEADER: 'https://schema.dash.org/platform-4-0-0/system/st-packet-header',
};

module.exports = JsonSchemaValidator;
