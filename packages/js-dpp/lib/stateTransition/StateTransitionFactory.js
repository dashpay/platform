const { decode } = require('../util/serializer');

const InvalidStateTransitionError = require('./errors/InvalidStateTransitionError');
const SerializedObjectParsingError = require('../errors/SerializedObjectParsingError');

class StateTransitionFactory {
  /**
   * @param {validateStateTransitionBasic} validateStateTransitionBasic
   * @param {createStateTransition} createStateTransition
   */
  constructor(
    validateStateTransitionBasic,
    createStateTransition,
  ) {
    this.validateStateTransitionBasic = validateStateTransitionBasic;
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
      const result = await this.validateStateTransitionBasic(rawStateTransition);

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
      // first 4 bytes are protocol version
      rawStateTransition = decode(buffer.slice(4, buffer.length));
      rawStateTransition.protocolVersion = buffer.slice(0, 4).readUInt32BE(0);
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
