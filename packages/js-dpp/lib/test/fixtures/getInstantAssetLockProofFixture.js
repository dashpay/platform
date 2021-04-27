const {
  Transaction,
  InstantLock,
  PrivateKey,
  Script,
  Opcode,
} = require('@dashevo/dashcore-lib');
const InstantAssetLockProof = require('../../identity/stateTransitions/assetLockProof/instant/InstantAssetLockProof');

/**
 * @param {PrivateKey} [oneTimePrivateKey]
 */
function getInstantAssetLockProofFixture(oneTimePrivateKey = new PrivateKey()) {
  const privateKeyHex = 'cSBnVM4xvxarwGQuAfQFwqDg9k5tErHUHzgWsEfD4zdwUasvqRVY';
  const privateKey = new PrivateKey(privateKeyHex);
  const fromAddress = privateKey.toAddress();

  const oneTimePublicKey = oneTimePrivateKey.toPublicKey();

  const transaction = new Transaction()
    .from({
      address: fromAddress,
      txId: 'a477af6b2667c29670467e4e0728b685ee07b240235771862318e29ddbe58458',
      outputIndex: 0,
      script: Script.buildPublicKeyHashOut(fromAddress)
        .toString(),
      satoshis: 100000,
    })
    // eslint-disable-next-line no-underscore-dangle
    .addBurnOutput(90000, oneTimePublicKey._getID())
    .to(fromAddress, 5000)
    .addOutput(Transaction.Output({
      satoshis: 5000,
      script: Script()
        .add(Opcode.OP_RETURN)
        .add(Buffer.from([1, 2, 3])),
    }))
    .sign(privateKey);

  const instantLock = new InstantLock({
    inputs: [
      {
        outpointHash: '6e200d059fb567ba19e92f5c2dcd3dde522fd4e0a50af223752db16158dabb1d',
        outpointIndex: 0,
      },
    ],
    txid: transaction.id,
    signature: '8967c46529a967b3822e1ba8a173066296d02593f0f59b3a78a30a7eef9c8a120847729e62e4a32954339286b79fe7590221331cd28d576887a263f45b595d499272f656c3f5176987c976239cac16f972d796ad82931d532102a4f95eec7d80',
  });

  return new InstantAssetLockProof({
    type: 0,
    instantLock: instantLock.toBuffer(),
    transaction: transaction.toBuffer(),
    outputIndex: 0,
  });
}

module.exports = getInstantAssetLockProofFixture;
