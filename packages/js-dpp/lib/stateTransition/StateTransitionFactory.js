const { decode } = require('../util/serializer');

const InvalidStateTransitionError = require('./errors/InvalidStateTransitionError');

class StateTransitionFactory {
  /**
   * @param {validateStateTransitionStructure} validateStateTransitionStructure
   * @param {createStateTransition} createStateTransition
   */
  constructor(validateStateTransitionStructure, createStateTransition) {
    this.validateStateTransitionStructure = validateStateTransitionStructure;
    this.createStateTransition = createStateTransition;
  }

  /**
   * Create State Transition from plain object
   *
   * @param {RawDataContractStateTransition|RawDocumentsStateTransition} rawStateTransition
   * @param {Object} options
   * @param {boolean} [options.skipValidation=false]
   * @return {DataContractStateTransition|DocumentsStateTransition}
   */
  async createFromObject(rawStateTransition, options = {}) {
    const opts = Object.assign({ skipValidation: false }, options);

    if (!opts.skipValidation) {
      const result = await this.validateStateTransitionStructure(rawStateTransition);

      if (!result.isValid()) {
        throw new InvalidStateTransitionError(result.getErrors(), rawStateTransition);
      }
    }

    return this.createStateTransition(rawStateTransition);
  }

  /**
   * Create State Transition from string/buffer
   *
   * @param {Buffer|string} payload
   * @param {Object} options
   * @param {boolean} [options.skipValidation=false]
   * @return {DataContractStateTransition|DocumentsStateTransition}
   */
  async createFromSerialized(payload, options = { }) {
    const rawStateTransition = decode(payload);

    return this.createFromObject(rawStateTransition, options);
  }
}

module.exports = StateTransitionFactory;
