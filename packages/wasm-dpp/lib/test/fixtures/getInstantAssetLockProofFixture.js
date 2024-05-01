const {
  Transaction,
  InstantLock,
  PrivateKey,
  Script,
  Opcode,
} = require('@dashevo/dashcore-lib');

const { default: loadWasmDpp } = require('../../..');
let { InstantAssetLockProof } = require('../../..');

/**
 * @param {PrivateKey} [oneTimePrivateKey]
 */
async function getInstantAssetLockProofFixture(oneTimePrivateKey = new PrivateKey()) {
  ({ InstantAssetLockProof } = await loadWasmDpp());

  const privateKeyHex = 'cSBnVM4xvxarwGQuAfQFwqDg9k5tErHUHzgWsEfD4zdwUasvqRVY';
  const privateKey = new PrivateKey(privateKeyHex);
  const fromAddress = privateKey.toAddress();

  const transaction = new Transaction(undefined);

  const output = {
    satoshis: 100000,
    address: fromAddress.toString(),
  };

  const realOutput = {
    satoshis: output.satoshis,
    script: Script
      .buildPublicKeyHashOut(oneTimePrivateKey.toAddress()).toString(),
  };

  const payload = Transaction.Payload.AssetLockPayload.fromJSON({
    version: 1,
    creditOutputs: [{
      satoshis: realOutput.satoshis,
      script: realOutput.script,
    }],
  });

  transaction
    .setType(Transaction.TYPES.TRANSACTION_ASSET_LOCK)
    .from({
      address: fromAddress,
      txId: 'a477af6b2667c29670467e4e0728b685ee07b240235771862318e29ddbe58458',
      outputIndex: 0,
      script: Script.buildPublicKeyHashOut(fromAddress)
        .toString(),
      satoshis: 100000,
    })
    .addOutput(
      new Transaction.Output({
        satoshis: realOutput.satoshis,
        // @ts-ignore
        script: new Script().add(Opcode.OP_RETURN).add(Buffer.alloc(0)),
      }),
    )
    // .change(changeAddress)
    // @ts-ignore
    .setExtraPayload(payload)
    .sign(privateKey);

  const instantLock = new InstantLock({
    version: 1,
    inputs: [
      {
        outpointHash: '6e200d059fb567ba19e92f5c2dcd3dde522fd4e0a50af223752db16158dabb1d',
        outpointIndex: 0,
      },
    ],
    txid: transaction.id,
    cyclehash: '7c30826123d0f29fe4c4a8895d7ba4eb469b1fafa6ad7b23896a1a591766a536',
    signature: '8967c46529a967b3822e1ba8a173066296d02593f0f59b3a78a30a7eef9c8a120847729e62e4a32954339286b79fe7590221331cd28d576887a263f45b595d499272f656c3f5176987c976239cac16f972d796ad82931d532102a4f95eec7d80',
  });

  return new InstantAssetLockProof({
    instantLock: instantLock.toBuffer(),
    transaction: transaction.toBuffer(),
    outputIndex: 0,
  });
}

module.exports = getInstantAssetLockProofFixture;
