const {
  Transaction,
  PrivateKey,
  PublicKey,
} = require('@dashevo/dashcore-lib');

const wait = require('../wait');

/**
 *
 * @param {DAPIClient} dapiClient
 * @param {Address} faucetAddress
 * @param {PrivateKey} faucetPrivateKey
 * @param {Address} address
 * @return {Promise<string>}
 */
async function fundAddress(dapiClient, faucetAddress, faucetPrivateKey, address) {
  const { items: inputs } = await dapiClient.getUTXO(faucetAddress.toString());

  const transaction = new Transaction();

  transaction.from(inputs.slice(-1)[0])
    .to(address, 20000)
    .change(faucetAddress)
    .fee(668)
    .sign(faucetPrivateKey);

  const transactionId = await dapiClient.sendTransaction(transaction.toBuffer());

  await dapiClient.generateToAddress(1, faucetAddress.toString());
  await wait(5000);

  return transactionId;
}

/**
 *
 * @param {DashPlatformProtocol} dpp
 * @param {DAPIClient} dapiClient
 * @param {PrivateKey} privateKey
 * @return {Promise<Identity>}
 */
async function createIdentity(dpp, dapiClient, privateKey) {
  const network = process.env.NETWORK === 'devnet' ? 'testnet' : process.env.NETWORK;

  // Prepare keys for a new Identity
  const publicKey = new PublicKey({
    ...privateKey.toPublicKey().toObject(),
    compressed: true,
  });
  const publicKeyBase = publicKey.toBuffer().toString('base64');

  // eslint-disable-next-line no-underscore-dangle
  const publicKeyHash = PublicKey.fromBuffer(Buffer.from(publicKeyBase, 'base64'))._getID();

  // Found and create a lock transaction
  const lockPrivateKey = new PrivateKey();
  const lockAddress = lockPrivateKey.toAddress(network);

  // Found the lock address
  const faucetPrivateKey = new PrivateKey(process.env.FAUCET_PRIVATE_KEY);
  const faucetAddress = faucetPrivateKey.toAddress(network);

  await fundAddress(dapiClient, faucetAddress, faucetPrivateKey, lockAddress);

  // Create a Lock Transaction
  const { items: [input] } = await dapiClient.getUTXO(lockAddress.toString());

  const lockTransaction = new Transaction();

  lockTransaction.from(input)
    .addBurnOutput(10000, publicKeyHash)
    .change(faucetAddress)
    .fee(668)
    .sign(lockPrivateKey);

  await dapiClient.sendTransaction(lockTransaction.toBuffer());

  await dapiClient.generateToAddress(1, faucetAddress.toString());
  await wait(5000);

  const outPoint = lockTransaction.getOutPointBuffer(0);

  // Apply Identity Create Transition
  const identity = dpp.identity.create(outPoint, [publicKey]);

  const identityCreateTransition = dpp.identity.createIdentityCreateTransition(identity);

  identityCreateTransition.signByPrivateKey(privateKey);

  await dapiClient.applyStateTransition(identityCreateTransition);

  // Get Identity back
  const identityBuffer = await dapiClient.getIdentity(identityCreateTransition.getIdentityId());

  return dpp.identity.createFromSerialized(identityBuffer);
}

module.exports = createIdentity;
