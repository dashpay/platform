const dashSchema = require('../../schema/meta/dash-schema');
const dpContractMetaSchema = require('../../schema/meta/dp-contract');
const stPacketHeaderSchema = require('../../schema/st-packet-header');

const ValidationResult = require('./ValidationResult');
const JsonSchemaError = require('../errors/JsonSchemaError');

class JsonSchemaValidator {
  constructor(ajv) {
    this.ajv = ajv;

    // TODO Validator shouldn't know about schemas

    this.ajv.addMetaSchema(dashSchema);

    this.ajv.addSchema(stPacketHeaderSchema);

    this.ajv.addMetaSchema(dpContractMetaSchema);
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
    DP_CONTRACT: 'https://schema.dash.org/dpp-0-4-0/meta/dp-contract',
  },
  BASE: {
    DP_OBJECT: 'https://schema.dash.org/dpp-0-4-0/base/document',
    ST_PACKET: 'https://schema.dash.org/dpp-0-4-0/base/st-packet',
  },
  ST_PACKET_HEADER: 'https://schema.dash.org/dpp-0-4-0/st-packet-header',
};

module.exports = JsonSchemaValidator;
