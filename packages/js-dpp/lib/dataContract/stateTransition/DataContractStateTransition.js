const AbstractStateTransition = require('../../stateTransition/AbstractStateTransition');
const stateTransitionTypes = require('../../stateTransition/stateTransitionTypes');

class DataContractStateTransition extends AbstractStateTransition {
  /**
   * @param {DataContract} dataContract
   */
  constructor(dataContract) {
    super();

    this.setDataContract(dataContract);
  }

  /**
   * Get State Transition type
   *
   * @return {number}
   */
  getType() {
    return stateTransitionTypes.DATA_CONTRACT;
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
   * @return {DataContractStateTransition}
   */
  setDataContract(dataContract) {
    this.dataContract = dataContract;

    return this;
  }

  /**
   * Get Data Contract State Transition as plain object
   *
   * @params {Object} [options]
   * @return {RawDataContractStateTransition}
   */
  toJSON(options = {}) {
    return {
      ...super.toJSON(options),
      dataContract: this.getDataContract().toJSON(),
    };
  }
}

module.exports = DataContractStateTransition;
