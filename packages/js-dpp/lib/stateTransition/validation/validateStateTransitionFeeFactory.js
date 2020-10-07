const ValidationResult = require('../../validation/ValidationResult');

const InvalidStateTransitionTypeError = require('../../errors/InvalidStateTransitionTypeError');
const BalanceIsNotEnoughError = require('../../errors/BalanceIsNotEnoughError');
const ConsensusError = require('../../errors/ConsensusError');

const stateTransitionTypes = require('../stateTransitionTypes');
const { convertSatoshiToCredits } = require('../../identity/creditsConverter');
const calculateStateTransitionFee = require('../calculateStateTransitionFee');

/**
 * Validate state transition fee
 *
 * @param {StateRepository} stateRepository
 * @param {fetchConfirmedAssetLockTransactionOutput} fetchConfirmedAssetLockTransactionOutput
 * @return {validateStateTransitionFee}
 */
function validateStateTransitionFeeFactory(
  stateRepository,
  fetchConfirmedAssetLockTransactionOutput,
) {
  /**
   * @typedef validateStateTransitionFee
   * @param {AbstractStateTransition} stateTransition
   * @return {ValidationResult}
   */
  async function validateStateTransitionFee(stateTransition) {
    const result = new ValidationResult();

    const feeSize = calculateStateTransitionFee(stateTransition);

    let balance;

    switch (stateTransition.getType()) {
      case stateTransitionTypes.IDENTITY_TOP_UP:
      case stateTransitionTypes.IDENTITY_CREATE: {
        let output;
        try {
          output = await fetchConfirmedAssetLockTransactionOutput(
            stateTransition.getLockedOutPoint(),
          );
        } catch (e) {
          if (e instanceof ConsensusError) {
            result.addError(e);
          } else {
            throw e;
          }
        }

        if (!result.isValid()) {
          return result;
        }

        balance = convertSatoshiToCredits(output.satoshis);

        if (stateTransition.getType() === stateTransitionTypes.IDENTITY_TOP_UP) {
          const identityId = stateTransition.getOwnerId();
          const identity = await stateRepository.fetchIdentity(identityId);
          balance += identity.getBalance();
        }

        break;
      }
      case stateTransitionTypes.DATA_CONTRACT_CREATE:
      case stateTransitionTypes.DOCUMENTS_BATCH: {
        const identityId = stateTransition.getOwnerId();
        const identity = await stateRepository.fetchIdentity(identityId);
        balance = identity.getBalance();

        break;
      }
      default:
        throw new InvalidStateTransitionTypeError(stateTransition.toObject());
    }

    if (balance < feeSize) {
      result.addError(
        new BalanceIsNotEnoughError(balance),
      );
    }

    return result;
  }

  return validateStateTransitionFee;
}

module.exports = validateStateTransitionFeeFactory;
