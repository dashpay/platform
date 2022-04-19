const ValidationResult = require('../../../../../validation/ValidationResult');

const identitySchema = require('../../../../../../schema/identity/identity.json');

const DuplicatedIdentityPublicKeyError = require(
  '../../../../../errors/consensus/state/identity/DuplicatedIdentityPublicKeyError',
);
const DuplicatedIdentityPublicKeyIdError = require(
  '../../../../../errors/consensus/state/identity/DuplicatedIdentityPublicKeyIdError',
);

const MaxIdentityPublicKeyLimitReachedError = require(
  '../../../../../errors/consensus/state/identity/MaxIdentityPublicKeyLimitReachedError',
);

/**
   * Validate public keys
   *
   * @typedef validatePublicKeysState
   *
   * @param {RawIdentityPublicKey[]} rawPublicKeys
   *
   * @return {ValidationResult}
   */
function validatePublicKeysState(rawPublicKeys) {
  const result = new ValidationResult();

  if (rawPublicKeys.length > identitySchema.properties.publicKeys.maxItems) {
    result.addError(
      new MaxIdentityPublicKeyLimitReachedError(identitySchema.properties.publicKeys.maxItems),
    );

    return result;
  }

  // Check that there's no duplicated key ids in the state transition
  const duplicatedIds = [];
  const idsCount = {};

  rawPublicKeys.forEach((rawPublicKey) => {
    idsCount[rawPublicKey.id] = !idsCount[rawPublicKey.id] ? 1 : idsCount[rawPublicKey.id] + 1;
    if (idsCount[rawPublicKey.id] > 1) {
      duplicatedIds.push(rawPublicKey.id);
    }
  });

  if (duplicatedIds.length > 0) {
    result.addError(
      new DuplicatedIdentityPublicKeyIdError(duplicatedIds),
    );
  }

  // Check that there's no duplicated keys
  const keysCount = {};
  const duplicatedKeyIds = [];
  rawPublicKeys
    .filter((rawPublicKey) => rawPublicKey.disabledAt === undefined)
    .forEach((rawPublicKey) => {
      const dataHex = rawPublicKey.data.toString('hex');

      keysCount[dataHex] = !keysCount[dataHex]
        ? 1 : keysCount[dataHex] + 1;

      if (keysCount[dataHex] > 1) {
        duplicatedKeyIds.push(rawPublicKey.id);
      }
    });

  if (duplicatedKeyIds.length > 0) {
    result.addError(
      new DuplicatedIdentityPublicKeyError(duplicatedKeyIds),
    );
  }

  return result;
}

module.exports = validatePublicKeysState;
