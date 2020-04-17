const dataContractMetaSchema = require('../../schema/dataContract/dataContractMeta');

const ValidationResult = require('./ValidationResult');
const JsonSchemaError = require('../errors/JsonSchemaError');

class JsonSchemaValidator {
  constructor(ajv) {
    this.ajv = ajv;

    // TODO Validator shouldn't know about schemas
    this.ajv.addMetaSchema(dataContractMetaSchema);
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
      (this.ajv.errors || []).map((error) => new JsonSchemaError(error)),
    );
  }

  /**
   * Validate JSON Schema
   *
   * @param {object} schema
   * @param additionalSchemas
   * @return {ValidationResult}
   */
  validateSchema(schema, additionalSchemas = {}) {
    const result = new ValidationResult();

    Object.keys(additionalSchemas).forEach((schemaId) => {
      this.ajv.addSchema(additionalSchemas[schemaId], schemaId);
    });

    try {
      // TODO: Use validateSchema
      //  https://github.com/epoberezkin/ajv#validateschemaobject-schema---boolean
      this.ajv.compile(schema);
    } catch (e) {
      result.addError(
        new JsonSchemaError(e),
      );
    } finally {
      Object.keys(additionalSchemas).forEach((schemaId) => {
        this.ajv.removeSchema(schemaId);
      });
    }

    return result;
  }
}

JsonSchemaValidator.SCHEMAS = {
  META: {
    DATA_CONTRACT: 'https://schema.dash.org/dpp-0-4-0/meta/data-contract',
  },
  BASE: {
    DP_OBJECT: 'https://schema.dash.org/dpp-0-4-0/base/document',
  },
};

module.exports = JsonSchemaValidator;
