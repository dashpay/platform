const lodashCloneDeepWith = require('lodash.clonedeepwith');
const lodashGet = require('lodash.get');
const lodashSet = require('lodash.set');

const cloneDeepWithIdentifiers = require('../../../../util/cloneDeepWithIdentifiers');

const AbstractDocumentTransition = require('./AbstractDocumentTransition');

const Identifier = require('../../../../identifier/Identifier');

/**
 * @abstract
 */
class AbstractDataDocumentTransition extends AbstractDocumentTransition {
  /**
   * @param {RawDocumentCreateTransition} rawTransition
   * @param {DataContract} dataContract
   */
  constructor(rawTransition, dataContract) {
    super(rawTransition, dataContract);

    const binaryProperties = dataContract.getBinaryProperties(
      this.getType(),
    );

    const identifierProperties = Object.fromEntries(
      Object.entries(binaryProperties)
        .filter(([, property]) => property.contentMediaType === Identifier.MEDIA_TYPE),
    );

    const data = Object.fromEntries(
      Object.entries(rawTransition).filter(([propertyName]) => !propertyName.startsWith('$')),
    );

    this.data = cloneDeepWithIdentifiers(data);

    Object.keys(identifierProperties)
      .forEach((propertyPath) => {
        const value = lodashGet(this.data, propertyPath);

        if (value !== undefined) {
          const clonedValue = Identifier.from(value);

          lodashSet(
            this.data,
            propertyPath,
            clonedValue,
          );
        }
      });
  }

  /**
   * Get data
   *
   * @returns {Object}
   */
  getData() {
    return this.data;
  }

  /**
   * Get plain object representation
   *
   * @param {Object} [options]
   * @param {boolean} [options.skipIdentifiersConversion=false]
   * @return {RawDocumentCreateTransition}
   */
  toObject(options = {}) {
    Object.assign(
      options,
      {
        skipIdentifiersConversion: false,
        ...options,
      },
    );

    const rawDocumentTransition = {
      ...super.toObject(options),
      ...this.getData(),
    };

    if (!options.skipIdentifiersConversion) {
      // eslint-disable-next-line consistent-return
      return lodashCloneDeepWith(rawDocumentTransition, (value) => {
        if (value instanceof Identifier) {
          return value.toBuffer();
        }
      });
    }

    return rawDocumentTransition;
  }

  /**
   * Get JSON representation
   *
   * @return {JsonDocumentCreateTransition}
   */
  toJSON() {
    const jsonDocumentTransition = super.toJSON();

    // eslint-disable-next-line consistent-return
    return lodashCloneDeepWith(jsonDocumentTransition, (value) => {
      if (value instanceof Identifier) {
        return value.toString();
      }

      if (Buffer.isBuffer(value)) {
        return value.toString('base64');
      }
    });
  }
}

module.exports = AbstractDataDocumentTransition;
