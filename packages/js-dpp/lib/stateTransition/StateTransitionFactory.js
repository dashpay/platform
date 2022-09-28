const InvalidStateTransitionError = require('./errors/InvalidStateTransitionError');
const AbstractConsensusError = require('../errors/consensus/AbstractConsensusError');
const StateTransitionExecutionContext = require('./StateTransitionExecutionContext');

class StateTransitionFactory {
  /**
   * @param {validateStateTransitionBasic} validateStateTransitionBasic
   * @param {createStateTransition} createStateTransition
   * @param {DashPlatformProtocol} dpp
   * @param {decodeProtocolEntity} decodeProtocolEntity
   */
  constructor(
    validateStateTransitionBasic,
    createStateTransition,
    dpp,
    decodeProtocolEntity,
  ) {
    this.validateStateTransitionBasic = validateStateTransitionBasic;
    this.createStateTransition = createStateTransition;
    this.dpp = dpp;
    this.decodeProtocolEntity = decodeProtocolEntity;
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

    const executionContext = new StateTransitionExecutionContext();

    if (!opts.skipValidation) {
      const result = await this.validateStateTransitionBasic(rawStateTransition, executionContext);

      if (!result.isValid()) {
        throw new InvalidStateTransitionError(result.getErrors(), rawStateTransition);
      }
    }

    // noinspection UnnecessaryLocalVariableJS
    const stateTransition = await this.createStateTransition(rawStateTransition, executionContext);

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
    let protocolVersion;

    try {
      [protocolVersion, rawStateTransition] = this.decodeProtocolEntity(
        buffer,
      );

      rawStateTransition.protocolVersion = protocolVersion;
    } catch (error) {
      if (error instanceof AbstractConsensusError) {
        throw new InvalidStateTransitionError([error]);
      }

      throw error;
    }

    return this.createFromObject(rawStateTransition, options);
  }
}

module.exports = StateTransitionFactory;
