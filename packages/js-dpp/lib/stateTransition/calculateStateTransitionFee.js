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
  const serializedStateTransition = stateTransition.serialize({ skipSignature: true });
  const byteSize = Buffer.byteLength(serializedStateTransition);
  return byteSize * PRICE_PER_BYTE;
}

module.exports = calculateStateTransitionFee;
