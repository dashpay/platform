const InvalidStateTransitionError = require('@dashevo/dpp/lib/stateTransition/errors/InvalidStateTransitionError');
const BalanceIsNotEnoughError = require('@dashevo/dpp/lib/errors/BalanceIsNotEnoughError');

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
      const errors = result.getErrors();
      if (errors.length === 1 && errors[0] instanceof BalanceIsNotEnoughError) {
        throw new InsufficientFundsError(errors[0].getBalance());
      } else {
        throw new InvalidArgumentAbciError('State Transition is invalid', { errors: result.getErrors() });
      }
    }

    return stateTransition;
  }

  return unserializeStateTransition;
}

module.exports = unserializeStateTransitionFactory;
