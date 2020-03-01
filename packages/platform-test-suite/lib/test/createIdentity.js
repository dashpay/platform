const {
  Transaction,
  PrivateKey,
  PublicKey,
} = require('@dashevo/dashcore-lib');

const DashPlatformProtocol = require('@dashevo/dpp');

const stateTransitionTypes = require('@dashevo/dpp/lib/stateTransition/stateTransitionTypes');
const IdentityCreateTransition = require('@dashevo/dpp/lib/identity/stateTransitions/identityCreateTransition/IdentityCreateTransition');
const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');

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
 * @param {DAPIClient} dapiClient
 * @param {string} outPoint
 * @param {number} type
 * @param {PrivateKey} privateKey
 * @param {IdentityPublicKey} identityPublicKey
 * @return {Promise<IdentityCreateTransition>}
 */
async function applyIdentityCreateTransition(
  dapiClient,
  outPoint,
  type,
  privateKey,
  identityPublicKey,
) {
  const identityCreateTransition = new IdentityCreateTransition({
    protocolVersion: 0,
    type: stateTransitionTypes.IDENTITY_CREATE,
    lockedOutPoint: outPoint,
    identityType: type,
    publicKeys: [
      identityPublicKey.toJSON(),
    ],
  });

  identityCreateTransition.sign(identityPublicKey, privateKey);

  await dapiClient.applyStateTransition(identityCreateTransition);

  return identityCreateTransition;
}

/**
 *
 * @param {DAPIClient} dapiClient
 * @param {PrivateKey} privateKey
 * @param {number} type
 * @return {Promise<Identity>}
 */
async function createIdentity(dapiClient, privateKey, type) {
  const network = process.env.NETWORK === 'devnet' ? 'testnet' : process.env.NETWORK;

  // Prepare keys for a new Identity
  const publicKey = new PublicKey({
    ...privateKey.toPublicKey().toObject(),
    compressed: true,
  });
  const publicKeyBase = publicKey.toBuffer().toString('base64');

  // eslint-disable-next-line no-underscore-dangle
  const publicKeyHash = PublicKey.fromBuffer(Buffer.from(publicKeyBase, 'base64'))._getID();

  const identityPublicKey = new IdentityPublicKey()
    .setId(1)
    .setType(IdentityPublicKey.TYPES.ECDSA_SECP256K1)
    .setData(publicKeyBase);

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

  const outPoint = lockTransaction.getOutPointBuffer(0)
    .toString('base64');

  // Apply Identity Create Transition
  const identityCreateTransition = await applyIdentityCreateTransition(
    dapiClient,
    outPoint,
    type,
    privateKey,
    identityPublicKey,
  );

  // Get Identity back
  const identityBuffer = await dapiClient.getIdentity(identityCreateTransition.getIdentityId());

  // Wrap into model
  const dpp = new DashPlatformProtocol();

  return dpp.identity.createFromSerialized(identityBuffer);
}

module.exports = createIdentity;
