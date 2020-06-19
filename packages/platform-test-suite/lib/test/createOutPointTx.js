const {
  Transaction,
} = require('@dashevo/dashcore-lib');
const { utils } = require('@dashevo/wallet-lib');

/**
 * @typedef createOutPointTx
 *
 * @param {number} amount
 * @param {Account} account
 *
 * @returns {Transaction}
 */
function createOutPointTx(
  amount,
  account,
) {
  const identityAddressInfo = account.getUnusedAddress();
  const [identityHDPrivateKey] = account.getPrivateKeys([identityAddressInfo.address]);
  const assetLockPrivateKey = identityHDPrivateKey.privateKey;
  const assetLockPublicKey = assetLockPrivateKey.toPublicKey();
  const identityAddress = assetLockPublicKey.toAddress(process.env.NETWORK).toString();
  const output = {
    satoshis: amount,
    address: identityAddress,
  };
  const outputToMarkItUsed = {
    satoshis: 10000,
    address: identityAddress,
  };
  const changeAddress = account.getUnusedAddress('internal').address;
  const utxos = account.getUTXOS();

  const selection = utils.coinSelection(utxos, [output, outputToMarkItUsed]);

  if (!selection.utxos.length) {
    throw new Error(`Address ${changeAddress} has no inputs to spend`);
  }

  const utxoAddresses = selection.utxos.map((utxo) => utxo.address.toString());
  const utxoHDPrivateKey = account.getPrivateKeys(utxoAddresses);
  const signingKeys = utxoHDPrivateKey.map((hdprivateKey) => hdprivateKey.privateKey);

  const outPointTx = new Transaction();

  outPointTx
    .from(selection.utxos)
    // eslint-disable-next-line no-underscore-dangle
    .addBurnOutput(output.satoshis, assetLockPublicKey._getID())
    .to(identityAddressInfo.address, 10000)
    .change(changeAddress)
    .sign(signingKeys);

  return {
    transaction: outPointTx,
    privateKey: assetLockPrivateKey,
  };
}

module.exports = createOutPointTx;
