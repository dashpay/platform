/**
 *
 * @param {createStateTransition} createStateTransition
 * @returns {validatePublicKeySignatures}
 */
const ValidationResult = require('../../validation/ValidationResult');
const InvalidIdentityKeySignatureError = require('../../errors/consensus/basic/identity/InvalidIdentityKeySignatureError');

/**
 * @param {IdentityCreateTransition|IdentityUpdateTransition} stateTransition
 * @param {RawIdentityPublicKey[]} rawPublicKeys
 * @param {number} [i=0]
 * @returns {Promise<RawIdentityPublicKey>}
 */
async function verifyPublicKeysSequentially(stateTransition, rawPublicKeys, i = 0) {
  const rawPublicKey = rawPublicKeys[i];

  stateTransition.setSignature(rawPublicKey.signature);

  const result = await stateTransition.verifyByPublicKey(
    rawPublicKey.data,
    rawPublicKey.type,
  );

  if (!result) {
    return rawPublicKey;
  }

  // eslint-disable-next-line no-param-reassign
  if (rawPublicKeys.length > ++i) {
    return verifyPublicKeysSequentially(stateTransition, rawPublicKeys, i);
  }

  return undefined;
}

function validatePublicKeySignaturesFactory(createStateTransition) {
  /**
   * @typedef {validatePublicKeySignatures}
   * @param {RawStateTransition} rawStateTransition
   * @param {RawIdentityPublicKey[]} rawPublicKeys
   * @returns {Promise<ValidationResult>}
   */
  async function validatePublicKeySignatures(rawStateTransition, rawPublicKeys) {
    const stateTransition = await createStateTransition(rawStateTransition);

    const result = new ValidationResult();

    const invalidRawPublicKey = await verifyPublicKeysSequentially(stateTransition, rawPublicKeys);

    if (invalidRawPublicKey) {
      result.addError(
        new InvalidIdentityKeySignatureError(invalidRawPublicKey.id),
      );
    }

    return result;
  }

  return validatePublicKeySignatures;
}

module.exports = validatePublicKeySignaturesFactory;
