const PRICE_PER_BYTE = 1;

/**
 * Get State Transition fee size
 *
 * @typedef calculateStateTransitionFee
 * @param { DataContractCreateTransition|
 * DocumentsBatchTransition|
 * IdentityCreateTransition} stateTransition
 * @return {number}
 */
function calculateStateTransitionFee(stateTransition) {
  const serializedStateTransition = stateTransition.toBuffer({ skipSignature: true });
  return serializedStateTransition.length * PRICE_PER_BYTE;
}

module.exports = calculateStateTransitionFee;
module.exports.PRICE_PER_BYTE = PRICE_PER_BYTE;
