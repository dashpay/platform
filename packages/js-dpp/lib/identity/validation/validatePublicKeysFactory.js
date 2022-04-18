const lodashCloneDeep = require('lodash.clonedeep');

const { PublicKey } = require('@dashevo/dashcore-lib');

const ValidationResult = require('../../validation/ValidationResult');

const convertBuffersToArrays = require('../../util/convertBuffersToArrays');

const publicKeySchema = require('../../../schema/identity/publicKey.json');

const InvalidIdentityPublicKeyDataError = require(
  '../../errors/consensus/basic/identity/InvalidIdentityPublicKeyDataError',
);

const DuplicatedIdentityPublicKeyError = require(
  '../../errors/consensus/basic/identity/DuplicatedIdentityPublicKeyError',
);
const DuplicatedIdentityPublicKeyIdError = require(
  '../../errors/consensus/basic/identity/DuplicatedIdentityPublicKeyIdError',
);

const InvalidIdentityPublicKeySecurityLevelError = require(
  '../../errors/consensus/basic/identity/InvalidIdentityPublicKeySecurityLevelError',
);

const IdentityPublicKey = require('../IdentityPublicKey');

const publicKeyEnabledSchema = lodashCloneDeep(publicKeySchema);
delete publicKeyEnabledSchema.properties.disabledAt;

/**
 * Validate public keys (factory)
 *
 * @param {JsonSchemaValidator} validator
 * @param {BlsSignatures} bls
 *
 * @return {validatePublicKeys}
 */
function validatePublicKeysFactory(validator, bls) {
  /**
   * Validate public keys
   *
   * @typedef validatePublicKeys
   *
   * @param {RawIdentityPublicKey[]} rawPublicKeys
   * @param {{mustBeEnabled: boolean}} [options]
   *
   * @return {ValidationResult}
   */
  function validatePublicKeys(rawPublicKeys, options = {}) {
    const result = new ValidationResult();

    const { mustBeEnabled } = options;

    let schemaToValidate = publicKeySchema;

    if (mustBeEnabled) {
      schemaToValidate = publicKeyEnabledSchema;
    }

    // Validate public key structure
    rawPublicKeys.forEach((rawPublicKey) => {
      result.merge(
        validator.validate(
          schemaToValidate,
          convertBuffersToArrays(rawPublicKey),
        ),
      );
    });

    if (!result.isValid()) {
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
    rawPublicKeys.forEach((rawPublicKey) => {
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

    // validate key data
    rawPublicKeys
      .forEach((rawPublicKey) => {
        let validationError;

        switch (rawPublicKey.type) {
          case IdentityPublicKey.TYPES.ECDSA_SECP256K1: {
            const dataHex = rawPublicKey.data.toString('hex');

            if (!PublicKey.isValid(dataHex)) {
              validationError = PublicKey.getValidationError(dataHex);
            }
            break;
          }
          case IdentityPublicKey.TYPES.BLS12_381: {
            try {
              bls.PublicKey.fromBytes(
                Uint8Array.from(rawPublicKey.data),
              );
            } catch (e) {
              validationError = new TypeError('Invalid public key');
            }
            break;
          }
          case IdentityPublicKey.TYPES.ECDSA_HASH160:
          case IdentityPublicKey.TYPES.BIP13_SCRIPT_HASH:
            // TODO Validate that it's valid Script

            break;
          default:
            throw new TypeError(`Unknown public key type: ${rawPublicKey.type}`);
        }

        if (validationError !== undefined) {
          const consensusError = new InvalidIdentityPublicKeyDataError(
            rawPublicKey.id,
            validationError.message,
          );

          consensusError.setValidationError(validationError);

          result.addError(consensusError);
        }
      });

    // Validate that public keys have correct purpose and security level
    rawPublicKeys
      .forEach((rawPublicKey) => {
        const keyPurpose = rawPublicKey.purpose;
        const allowedSecurityLevels = IdentityPublicKey.ALLOWED_SECURITY_LEVELS[keyPurpose];

        if (!allowedSecurityLevels || !allowedSecurityLevels.includes(rawPublicKey.securityLevel)) {
          const error = new InvalidIdentityPublicKeySecurityLevelError(
            rawPublicKey.id,
            rawPublicKey.purpose,
            rawPublicKey.securityLevel,
            allowedSecurityLevels,
          );

          result.addError(error);
        }
      });

    return result;
  }

  return validatePublicKeys;
}

module.exports = validatePublicKeysFactory;
