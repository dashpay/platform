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
   * Create State Transition from JSON
   *
   * @param {RawDataContractCreateTransition|RawDocumentsBatchTransition} rawStateTransition
   * @param {Object} options
   * @param {boolean} [options.skipValidation=false]
   * @return {RawDataContractCreateTransition|DocumentsBatchTransition}
   */
  async createFromJSON(rawStateTransition, options = {}) {
    const opts = { skipValidation: false, ...options };

    if (!opts.skipValidation) {
      const result = await this.validateStateTransitionStructure(rawStateTransition);

      if (!result.isValid()) {
        throw new InvalidStateTransitionError(result.getErrors(), rawStateTransition);
      }
    }

    // noinspection UnnecessaryLocalVariableJS
    const stateTransition = await this.createStateTransition(rawStateTransition, {
      fromJSON: true,
    });

    return stateTransition;
  }

  /**
   * Create State Transition from plain object
   *
   * @param {RawDataContractCreateTransition|RawDocumentsBatchTransition} rawStateTransition
   * @param {Object} options
   * @param {boolean} [options.skipValidation=false]
   * @return {RawDataContractCreateTransition|DocumentsBatchTransition}
   */
  async createFromObject(rawStateTransition, options = {}) {
    const opts = { skipValidation: false, ...options };

    // noinspection UnnecessaryLocalVariableJS
    const stateTransition = await this.createStateTransition(rawStateTransition, {
      fromJSON: false,
    });

    if (!opts.skipValidation) {
      const result = await this.validateStateTransitionStructure(stateTransition.toJSON());

      if (!result.isValid()) {
        throw new InvalidStateTransitionError(result.getErrors(), rawStateTransition);
      }
    }

    return stateTransition;
  }

  /**
   * Create State Transition from string/buffer
   *
   * @param {Buffer|string} payload
   * @param {Object} options
   * @param {boolean} [options.skipValidation=false]
   * @return {RawDataContractCreateTransition|DocumentsBatchTransition}
   */
  async createFromSerialized(payload, options = { }) {
    let rawStateTransition;
    try {
      rawStateTransition = decode(payload);
    } catch (error) {
      throw new InvalidStateTransitionError([
        new SerializedObjectParsingError(
          payload,
          error,
        ),
      ]);
    }

    return this.createFromObject(rawStateTransition, options);
  }
}

module.exports = StateTransitionFactory;
