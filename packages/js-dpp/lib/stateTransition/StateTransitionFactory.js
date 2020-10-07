const { decode } = require('../util/serializer');

const InvalidStateTransitionError = require('./errors/InvalidStateTransitionError');
const SerializedObjectParsingError = require('../errors/SerializedObjectParsingError');

class StateTransitionFactory {
  /**
   * @param {validateStateTransitionStructure} validateStateTransitionStructure
   * @param {createStateTransition} createStateTransition
   */
  constructor(
    validateStateTransitionStructure,
    createStateTransition,
  ) {
    this.validateStateTransitionStructure = validateStateTransitionStructure;
    this.createStateTransition = createStateTransition;
  }

  /**
   * Create State Transition from plain object
   *
   * @param {RawStateTransition} rawStateTransition
   * @param {Object} [options]
   * @param {boolean} [options.skipValidation=false]
   * @return {AbstractStateTransition}
   */
  async createFromObject(rawStateTransition, options = {}) {
    const opts = { skipValidation: false, ...options };

    if (!opts.skipValidation) {
      const result = await this.validateStateTransitionStructure(rawStateTransition);

      if (!result.isValid()) {
        throw new InvalidStateTransitionError(result.getErrors(), rawStateTransition);
      }
    }

    // noinspection UnnecessaryLocalVariableJS
    const stateTransition = await this.createStateTransition(rawStateTransition);

    return stateTransition;
  }

  /**
   * Create State Transition from buffer
   *
   * @param {Buffer} buffer
   * @param {Object} options
   * @param {boolean} [options.skipValidation=false]
   * @return {RawDataContractCreateTransition|DocumentsBatchTransition}
   */
  async createFromBuffer(buffer, options = { }) {
    let rawStateTransition;
    try {
      rawStateTransition = decode(buffer);
    } catch (error) {
      throw new InvalidStateTransitionError([
        new SerializedObjectParsingError(
          buffer,
          error,
        ),
      ]);
    }

    return this.createFromObject(rawStateTransition, options);
  }
}

module.exports = StateTransitionFactory;
