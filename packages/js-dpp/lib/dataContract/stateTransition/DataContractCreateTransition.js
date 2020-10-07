const AbstractStateTransitionIdentitySigned = require('../../stateTransition/AbstractStateTransitionIdentitySigned');
const stateTransitionTypes = require('../../stateTransition/stateTransitionTypes');
const DataContract = require('../DataContract');
const EncodedBuffer = require('../../util/encoding/EncodedBuffer');

class DataContractCreateTransition extends AbstractStateTransitionIdentitySigned {
  /**
   * @param {RawDataContractCreateTransition} rawDataContractCreateTransition
   */
  constructor(rawDataContractCreateTransition) {
    super(rawDataContractCreateTransition);

    if (Object.prototype.hasOwnProperty.call(rawDataContractCreateTransition, 'entropy')) {
      this.entropy = EncodedBuffer.from(
        rawDataContractCreateTransition.entropy,
        EncodedBuffer.ENCODING.BASE58,
      );
    }

    const dataContract = new DataContract(rawDataContractCreateTransition.dataContract);

    this.setDataContract(dataContract);
  }

  /**
   * Get State Transition type
   *
   * @return {number}
   */
  getType() {
    return stateTransitionTypes.DATA_CONTRACT_CREATE;
  }

  /**
   * Get Data Contract
   *
   * @return {DataContract}
   */
  getDataContract() {
    return this.dataContract;
  }

  /**
   * Set Data Contract
   *
   * @param {DataContract} dataContract
   * @return {DataContractCreateTransition}
   */
  setDataContract(dataContract) {
    this.dataContract = dataContract;

    return this;
  }

  /**
   * Get entropy
   *
   * @returns {EncodedBuffer}
   */
  getEntropy() {
    return this.entropy;
  }

  /**
   * Get state transition as plain object
   *
   * @param {Object} [options]
   * @param {boolean} [options.skipSignature=false]
   * @param {boolean} [options.encodedBuffer=false]
   * @return {RawDataContractCreateTransition}
   */
  toObject(options = {}) {
    Object.assign(
      options,
      {
        encodedBuffer: false,
        ...options,
      },
    );

    const rawStateTransition = {
      ...super.toObject(options),
      dataContract: this.getDataContract().toObject(),
      entropy: this.getEntropy(),
    };

    if (!options.encodedBuffer) {
      rawStateTransition.entropy = this.getEntropy().toBuffer();
    }

    return rawStateTransition;
  }

  /**
   * Get state transition as JSON
   *
   * @return {JsonDataContractCreateTransition}
   */
  toJSON() {
    return {
      ...super.toJSON(),
      entropy: this.getEntropy().toString(),
      dataContract: this.getDataContract().toJSON(),
    };
  }

  /**
   * Get owner ID
   * @return {EncodedBuffer}
   */
  getOwnerId() {
    return this.getDataContract().getOwnerId();
  }
}

/**
 * @typedef {RawStateTransitionIdentitySigned & Object} RawDataContractCreateTransition
 * @property {RawDataContract} dataContract
 * @property {Buffer} entropy
 */

/**
 * @typedef {JsonStateTransitionIdentitySigned & Object} JsonDataContractCreateTransition
 * @property {JsonDataContract} dataContract
 * @property {string} entropy
 */

DataContractCreateTransition.ENCODED_PROPERTIES = {
  ...AbstractStateTransitionIdentitySigned.ENCODED_PROPERTIES,
  entropy: {
    contentEncoding: 'base58',
  },
};

module.exports = DataContractCreateTransition;
