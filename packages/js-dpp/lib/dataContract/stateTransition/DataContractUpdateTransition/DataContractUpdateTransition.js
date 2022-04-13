const AbstractStateTransitionIdentitySigned = require('../../../stateTransition/AbstractStateTransitionIdentitySigned');
const stateTransitionTypes = require('../../../stateTransition/stateTransitionTypes');
const DataContract = require('../../DataContract');

class DataContractUpdateTransition extends AbstractStateTransitionIdentitySigned {
  /**
   * @param {RawDataContractUpdateTransition} rawDataContractUpdateTransition
   */
  constructor(rawDataContractUpdateTransition) {
    super(rawDataContractUpdateTransition);

    const dataContract = new DataContract(rawDataContractUpdateTransition.dataContract);

    this.setDataContract(dataContract);
  }

  /**
   * Get State Transition type
   *
   * @return {number}
   */
  getType() {
    return stateTransitionTypes.DATA_CONTRACT_UPDATE;
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
   * @return {DataContractUpdateTransition}
   */
  setDataContract(dataContract) {
    this.dataContract = dataContract;

    return this;
  }

  /**
   * Get state transition as plain object
   *
   * @param {Object} [options]
   * @param {boolean} [options.skipSignature=false]
   * @param {boolean} [options.skipIdentifiersConversion=false]
   * @return {RawDataContractUpdateTransition}
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
    };
  }

  /**
   * Get state transition as JSON
   *
   * @return {JsonDataContractUpdateTransition}
   */
  toJSON() {
    return {
      ...super.toJSON(),
      dataContract: this.getDataContract().toJSON(),
    };
  }

  /**
   * Get owner ID
   * @return {Identifier}
   */
  getOwnerId() {
    return this.getDataContract().getOwnerId();
  }

  /**
   * Returns id of the created contract
   *
   * @return {Identifier[]}
   */
  getModifiedDataIds() {
    return [this.getDataContract().getId()];
  }
}

/**
 * @typedef {RawStateTransitionIdentitySigned & Object} RawDataContractUpdateTransition
 * @property {RawDataContract} dataContract
 */

/**
 * @typedef {JsonStateTransitionIdentitySigned & Object} JsonDataContractUpdateTransition
 * @property {JsonDataContract} dataContract
 */

module.exports = DataContractUpdateTransition;
