const AbstractDocumentTransition = require('./AbstractDocumentTransition');

const Identifier = require('../../../Identifier');

const transpileProperties = require('../../../util/transpileProperties');


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

    this.data = transpileProperties(
      data,
      identifierProperties,
      (value) => Identifier.from(value),
    );
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
      const binaryProperties = this.dataContract.getBinaryProperties(
        this.getType(),
      );

      const identifierProperties = Object.fromEntries(
        Object.entries(binaryProperties)
          .filter(([, property]) => property.contentMediaType === Identifier.MEDIA_TYPE),
      );

      return transpileProperties(
        rawDocumentTransition,
        identifierProperties,
        (value) => {
          if (value instanceof Identifier) {
            return value.toBuffer();
          }

          return value;
        },
      );
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

    const binaryProperties = this.dataContract.getBinaryProperties(
      this.getType(),
    );

    return transpileProperties(
      jsonDocumentTransition,
      binaryProperties,
      (value) => {
        if (value instanceof Identifier) {
          return value.toString();
        }

        if (Buffer.isBuffer(value)) {
          return value.toString('base64');
        }

        return value;
      },
    );
  }
}

module.exports = AbstractDataDocumentTransition;
