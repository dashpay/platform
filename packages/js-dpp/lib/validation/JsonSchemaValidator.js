const dataContractMetaSchema = require('../../schema/dataContract/dataContractMeta.json');

const ValidationResult = require('./ValidationResult');

const JsonSchemaError = require('../errors/consensus/basic/JsonSchemaError');
const JsonSchemaCompilationError = require('../errors/consensus/basic/JsonSchemaCompilationError');

class JsonSchemaValidator {
  constructor(ajv) {
    this.ajv = ajv;

    // TODO Validator shouldn't know about schemas
    this.ajv.addMetaSchema(dataContractMetaSchema);
    this.ajv.addVocabulary([
      'ownerId',
      'documents',
      'protocolVersion',
      'indices',
    ]);
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
      (this.ajv.errors || []).map((error) => new JsonSchemaError(
        error.message,
        error.keyword,
        error.instancePath,
        error.schemaPath,
        error.params,
        error.propertyName,
      )),
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
        new JsonSchemaCompilationError(e.message),
      );
    } finally {
      Object.keys(additionalSchemas).forEach((schemaId) => {
        this.ajv.removeSchema(schemaId);
      });
    }

    if (this.ajv.errors) {
      result.addError(
        this.ajv.errors.map((error) => new JsonSchemaError(
          error.message,
          error.keyword,
          error.instancePath,
          error.schemaPath,
          error.params,
          error.propertyName,
        )),
      );
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
