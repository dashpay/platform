const InvalidStateTransitionError = require('@dashevo/dpp/lib/stateTransition/errors/InvalidStateTransitionError');

const InvalidArgumentAbciError = require('../../errors/InvalidArgumentAbciError');
const MemoryLimitExceededError = require('../../errors/MemoryLimitExceededError');
const ExecutionTimedOutError = require('../../errors/ExecutionTimedOutError');
const InsufficientFundsError = require('../../errors/InsufficientFundsError');

/**
 * @param {createIsolatedDpp} createIsolatedDpp
 * @return {unserializeStateTransition}
 */
function unserializeStateTransitionFactory(createIsolatedDpp) {
  /**
   * @typedef unserializeStateTransition
   * @param {Uint8Array} stateTransitionByteArray
   * @return {DocumentsBatchTransition|DataContractCreateTransition|IdentityCreateTransition}
   */
  async function unserializeStateTransition(stateTransitionByteArray) {
    if (!stateTransitionByteArray) {
      throw new InvalidArgumentAbciError('State Transition is not specified');
    }

    const stateTransitionSerialized = Buffer.from(stateTransitionByteArray);

    const isolatedDpp = await createIsolatedDpp();

    let stateTransition;
    try {
      stateTransition = await isolatedDpp
        .stateTransition
        .createFromSerialized(stateTransitionSerialized);
    } catch (e) {
      if (e instanceof InvalidStateTransitionError) {
        throw new InvalidArgumentAbciError('State Transition is invalid', { errors: e.getErrors() });
      }

      if (e.message === 'Script execution timed out.') {
        throw new ExecutionTimedOutError();
      }

      if (e.message === 'Isolate was disposed during execution due to memory limit') {
        throw new MemoryLimitExceededError();
      }

      throw e;
    } finally {
      isolatedDpp.dispose();
    }

    const result = await isolatedDpp.stateTransition.validateFee(stateTransition);

    if (!result.isValid()) {
      const [error] = result.getErrors();

      throw new InsufficientFundsError(error.getBalance());
    }

    return stateTransition;
  }

  return unserializeStateTransition;
}

module.exports = unserializeStateTransitionFactory;
