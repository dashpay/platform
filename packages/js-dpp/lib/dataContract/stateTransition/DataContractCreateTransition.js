const AbstractStateTransitionIdentitySigned = require('../../stateTransition/AbstractStateTransitionIdentitySigned');
const stateTransitionTypes = require('../../stateTransition/stateTransitionTypes');
const DataContract = require('../DataContract');

class DataContractCreateTransition extends AbstractStateTransitionIdentitySigned {
  /**
   * @param {RawDataContractCreateTransition} rawDataContractCreateTransition
   */
  constructor(rawDataContractCreateTransition) {
    super(rawDataContractCreateTransition);

    if (Object.prototype.hasOwnProperty.call(rawDataContractCreateTransition, 'entropy')) {
      this.entropy = rawDataContractCreateTransition.entropy;
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
   * @returns {Buffer}
   */
  getEntropy() {
    return this.entropy;
  }

  /**
   * Get state transition as plain object
   *
   * @param {Object} [options]
   * @param {boolean} [options.skipSignature=false]
   * @param {boolean} [options.skipIdentifiersConversion=false]
   * @return {RawDataContractCreateTransition}
   */
  toObject(options = {}) {
    Object.assign(
      options,
      {
        skipIdentifiersConversion: false,
        ...options,
      },
    );

    return {
      ...super.toObject(options),
      dataContract: this.getDataContract().toObject(),
      entropy: this.getEntropy(),
    };
  }

  /**
   * Get state transition as JSON
   *
   * @return {JsonDataContractCreateTransition}
   */
  toJSON() {
    return {
      ...super.toJSON(),
      dataContract: this.getDataContract().toJSON(),
      entropy: this.getEntropy().toString('base64'),
    };
  }

  /**
   * Get owner ID
   * @return {Identifier}
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

module.exports = DataContractCreateTransition;
