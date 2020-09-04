const lodashGet = require('lodash.get');
const lodashSet = require('lodash.set');

const encodeToBase64WithoutPadding = require('../../../util/encodeToBase64WithoutPadding');

/**
 * @abstract
 */
class AbstractDocumentTransition {
  constructor(rawDocumentTransition, dataContract) {
    this.dataContractId = rawDocumentTransition.$dataContractId;
    this.dataContract = dataContract;
  }

  /**
   * @abstract
   */
  getAction() {
    throw new Error('Not implemented');
  }

  /**
   * Get Data Contract ID
   *
   * @return {string}
   */
  getDataContractId() {
    return this.dataContractId;
  }

  /**
   * Get plain object representation
   *
   * @return {Object}
   */
  toObject() {
    return {
      $action: this.getAction(),
      $dataContractId: this.getDataContractId(),
    };
  }

  /**
   * Get transition property by path
   *
   * @param {string} propertyPath
   *
   * @return {*}
   */
  get(propertyPath) {
    return lodashGet(this.getData(), propertyPath);
  }

  /**
   * Get JSON representation
   *
   * @return {Object}
   */
  toJSON() {
    const data = this.toObject();

    const encodedProperties = this.dataContract.getEncodedProperties(
      this.getType(),
    );

    Object.keys(encodedProperties)
      .forEach((propertyPath) => {
        const property = encodedProperties[propertyPath];

        if (property.contentEncoding === 'base64') {
          const value = lodashGet(data, propertyPath);
          if (value !== undefined) {
            lodashSet(
              data,
              propertyPath,
              encodeToBase64WithoutPadding(
                value,
              ),
            );
          }
        }
      });

    return data;
  }

  /**
   * Translate document transition from JSON to plain object
   *
   * @protected
   *
   * @param {
   *   RawDocumentCreateTransition | RawDocumentReplaceTransition | RawDocumentDeleteTransition
   * } rawDocumentTransition
   * @param {DataContract} dataContract
   *
   * @return {
   *   RawDocumentCreateTransition | RawDocumentReplaceTransition | RawDocumentDeleteTransition
   * }
   */
  static translateJsonToObject(rawDocumentTransition, dataContract) {
    const encodedProperties = dataContract.getEncodedProperties(
      rawDocumentTransition.$type,
    );

    Object.keys(encodedProperties)
      .forEach((propertyPath) => {
        const property = encodedProperties[propertyPath];

        if (property.contentEncoding === 'base64') {
          const value = lodashGet(rawDocumentTransition, propertyPath);
          if (value !== undefined) {
            lodashSet(
              rawDocumentTransition,
              propertyPath,
              Buffer.from(value, 'base64'),
            );
          }
        }
      });

    return rawDocumentTransition;
  }
}

AbstractDocumentTransition.ACTIONS = {
  CREATE: 0,
  REPLACE: 1,
  // 2 reserved for UPDATE
  DELETE: 3,
};

AbstractDocumentTransition.ACTION_NAMES = {
  CREATE: 'create',
  REPLACE: 'replace',
  DELETE: 'delete',
};

module.exports = AbstractDocumentTransition;
