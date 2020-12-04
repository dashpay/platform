const PrivateKey = require('@dashevo/dashcore-lib/lib/privatekey');
const IdentityCreateTransition = require('../../identity/stateTransitions/identityCreateTransition/IdentityCreateTransition');

const IdentityPublicKey = require('../../identity/IdentityPublicKey');

const stateTransitionTypes = require('../../stateTransition/stateTransitionTypes');

const getAssetLockFixture = require('./getAssetLockFixture');

/**
 * @param {PrivateKey} oneTimePrivateKey
 *
 * @return {IdentityCreateTransition}
 */
module.exports = function getIdentityCreateTransitionFixture(oneTimePrivateKey = new PrivateKey()) {
  const rawStateTransition = {
    protocolVersion: 0,
    type: stateTransitionTypes.IDENTITY_CREATE,
    assetLock: getAssetLockFixture(oneTimePrivateKey).toObject(),
    publicKeys: [
      {
        id: 0,
        type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
        data: Buffer.from('AuryIuMtRrl/VviQuyLD1l4nmxi9ogPzC9LT7tdpo0di', 'base64'),
      },
    ],
  };

  return new IdentityCreateTransition(rawStateTransition);
};
