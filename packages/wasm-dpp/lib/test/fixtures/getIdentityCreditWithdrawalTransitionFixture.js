const { Script, PrivateKey } = require('@dashevo/dashcore-lib');
const { IdentityCreditWithdrawalTransition } = require('../../..');
const generateRandomIdentifier = require('../utils/generateRandomIdentifier');

module.exports = function getIdentityCreditWithdrawalTransitionFixture() {
  const privateKey = new PrivateKey('cSBnVM4xvxarwGQuAfQFwqDg9k5tErHUHzgWsEfD4zdwUasvqRVY');
  const address = privateKey.toAddress();

  const stateTransition = new IdentityCreditWithdrawalTransition(1);
  stateTransition.setIdentityId(generateRandomIdentifier());
  // eslint-disable-next-line
  stateTransition.setAmount(BigInt(1000));
  stateTransition.setCoreFeePerByte(1000);
  stateTransition.setPooling(0);
  stateTransition.setOutputScript(Script.buildPublicKeyHashOut(address).toBuffer());
  // eslint-disable-next-line
  stateTransition.setNonce(BigInt(1));

  return stateTransition;
};
