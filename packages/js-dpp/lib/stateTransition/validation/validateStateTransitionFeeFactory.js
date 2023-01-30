const ValidationResult = require('../../validation/ValidationResult');

const InvalidStateTransitionTypeError = require('../errors/InvalidStateTransitionTypeError');
const BalanceIsNotEnoughError = require('../../errors/consensus/fee/BalanceIsNotEnoughError');

const stateTransitionTypes = require('../stateTransitionTypes');
const { convertSatoshiToCredits } = require('../../identity/creditsConverter');

/**
 * Validate state transition fee
 *
 * @param {StateRepository} stateRepository
 * @param {calculateStateTransitionFee} calculateStateTransitionFee
 * @param {fetchAssetLockTransactionOutput} fetchAssetLockTransactionOutput
 * @return {validateStateTransitionFee}
 */
function validateStateTransitionFeeFactory(
  stateRepository,
  calculateStateTransitionFee,
  fetchAssetLockTransactionOutput,
) {
  /**
   * @typedef validateStateTransitionFee
   * @param {AbstractStateTransition} stateTransition
   * @return {ValidationResult}
   */
  async function validateStateTransitionFee(stateTransition) {
    const result = new ValidationResult();

    const executionContext = stateTransition.getExecutionContext();

    let balance;
    switch (stateTransition.getType()) {
      case stateTransitionTypes.IDENTITY_TOP_UP:
      case stateTransitionTypes.IDENTITY_CREATE: {
        const output = await fetchAssetLockTransactionOutput(
          stateTransition.getAssetLockProof(),
          executionContext,
        );

        balance = convertSatoshiToCredits(output.satoshis);

        if (stateTransition.getType() === stateTransitionTypes.IDENTITY_TOP_UP) {
          const identityId = stateTransition.getOwnerId();

          const identityBalance = await stateRepository.fetchIdentityBalanceWithDebt(
            identityId,
            executionContext,
          );

          if (executionContext.isDryRun()) {
            return result;
          }

          balance += identityBalance;
        }

        break;
      }
      case stateTransitionTypes.DATA_CONTRACT_CREATE:
      case stateTransitionTypes.DATA_CONTRACT_UPDATE:
      case stateTransitionTypes.DOCUMENTS_BATCH:
      case stateTransitionTypes.IDENTITY_UPDATE: {
        const identityId = stateTransition.getOwnerId();

        const identityBalance = await stateRepository.fetchIdentityBalance(
          identityId,
          executionContext,
        );

        if (executionContext.isDryRun()) {
          return result;
        }

        balance = identityBalance;

        break;
      }
      default:
        throw new InvalidStateTransitionTypeError(stateTransition.getType());
    }

    if (executionContext.isDryRun()) {
      return result;
    }

    const { desiredAmount: fee } = calculateStateTransitionFee(stateTransition);

    if (balance < fee) {
      result.addError(
        new BalanceIsNotEnoughError(balance, fee),
      );
    }

    return result;
  }

  return validateStateTransitionFee;
}

module.exports = validateStateTransitionFeeFactory;
