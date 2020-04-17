const { PublicKey } = require('@dashevo/dashcore-lib');

const IdentityPublicKey = require('../IdentityPublicKey');

const ValidationResult = require('../../validation/ValidationResult');

const publicKeySchema = require('../../../schema/identity/publicKey.json');

const InvalidIdentityPublicKeyDataError = require(
  '../../errors/InvalidIdentityPublicKeyDataError',
);

const DuplicatedIdentityPublicKeyError = require(
  '../../errors/DuplicatedIdentityPublicKeyError',
);
const DuplicatedIdentityPublicKeyIdError = require(
  '../../errors/DuplicatedIdentityPublicKeyIdError',
);

/**
 * Validate public keys (factory)
 *
 * @param {JsonSchemaValidator} validator
 *
 * @return {validatePublicKeys}
 */
function validatePublicKeysFactory(validator) {
  /**
   * Validate public keys
   *
   * @typedef validatePublicKeys
   *
   * @param {IdentityPublicKey[]|RawIdentityPublicKey[]} publicKeys
   *
   * @return {ValidationResult}
   */
  function validatePublicKeys(publicKeys) {
    const result = new ValidationResult();

    // Convert everything to raw type
    const rawPublicKeys = publicKeys
      .map((publicKey) => {
        if (publicKey instanceof IdentityPublicKey) {
          return publicKey.toJSON();
        }

        return publicKey;
      });

    // Validate public key structure
    rawPublicKeys.forEach((publicKey) => {
      result.merge(
        validator.validate(publicKeySchema, publicKey),
      );
    });

    if (!result.isValid()) {
      return result;
    }

    // Check that there's no duplicated key ids in the state transition
    const duplicatedIds = [];
    const idsCount = {};

    publicKeys.forEach((publicKey) => {
      idsCount[publicKey.id] = !idsCount[publicKey.id] ? 1 : idsCount[publicKey.id] + 1;
      if (idsCount[publicKey.id] > 1) {
        duplicatedIds.push(publicKey.id);
      }
    });

    if (duplicatedIds.length > 0) {
      result.addError(new DuplicatedIdentityPublicKeyIdError(publicKeys));
    }

    // Check that there's no duplicated keys
    const keysCount = {};
    const duplicatedKeys = [];
    publicKeys.forEach((publicKey) => {
      keysCount[publicKey.data] = !keysCount[publicKey.data]
        ? 1 : keysCount[publicKey.data] + 1;
      if (keysCount[publicKey.data] > 1) {
        duplicatedKeys.push(publicKey.data);
      }
    });

    if (duplicatedKeys.length > 0) {
      result.addError(new DuplicatedIdentityPublicKeyError(publicKeys));
    }

    // validate key data
    rawPublicKeys
      .forEach((publicKey) => {
        const dataHex = Buffer.from(publicKey.data, 'base64')
          .toString('hex');

        if (!PublicKey.isValid(dataHex)) {
          result.addError(
            new InvalidIdentityPublicKeyDataError(
              publicKey,
              PublicKey.getValidationError(dataHex),
            ),
          );
        }
      });

    return result;
  }

  return validatePublicKeys;
}

module.exports = validatePublicKeysFactory;
