const {
  PrivateKey,
  PublicKey,
  Transaction,
} = require('@dashevo/dashcore-lib');

const IdentityPublicKey = require(
  '@dashevo/dpp/lib/identity/IdentityPublicKey',
);
const stateTransitionTypes = require(
  '@dashevo/dpp/lib/stateTransition/stateTransitionTypes',
);

const Identity = require('@dashevo/dpp/lib/identity/Identity');

const wait = require('../../lib/util/wait');

/**
 * Register user
 *
 * @param {RpcClient} coreAPI
 * @param {DapiClient} dapiClient
 * @param {DashPlatformProtocol} dpp
 *
 * @returns {Promise<{ identityId: string, identityPrivateKey: PrivateKey }>}
 */
async function registerUser(coreAPI, dapiClient, dpp) {
  let response = await coreAPI.getnewaddress();
  const address = response.result;

  response = await coreAPI.dumpprivkey(address);
  const privateKeyString = response.result;

  const privateKey = new PrivateKey(privateKeyString);
  const pubKeyBase = new PublicKey({
    ...privateKey.toPublicKey().toObject(),
    compressed: true,
  }).toBuffer()
    .toString('base64');
  // eslint-disable-next-line no-underscore-dangle
  const publicKeyHash = PublicKey.fromBuffer(Buffer.from(pubKeyBase, 'base64'))._getID();

  await coreAPI.generate(101);
  await coreAPI.sendtoaddress(address, 10);
  await coreAPI.generate(7);

  response = await coreAPI.listunspent();
  const unspent = response.result;
  const inputs = unspent.filter(input => input.address === address);

  const transaction = new Transaction();

  transaction.from(inputs)
    .addBurnOutput(10000, publicKeyHash)
    .change(address)
    .fee(668)
    .sign(privateKey);

  await coreAPI.sendrawtransaction(transaction.serialize());
  await coreAPI.generate(6);

  await wait(2000); // wait a couple of seconds for tx to be confirmed

  const outPoint = transaction.getOutPointBuffer(0)
    .toString('base64');

  const publicKeyId = 1;

  const identityCreateTransition = await dpp.stateTransition.createFromObject({
    protocolVersion: 0,
    type: stateTransitionTypes.IDENTITY_CREATE,
    lockedOutPoint: outPoint,
    identityType: Identity.TYPES.USER,
    publicKeys: [
      {
        id: publicKeyId,
        type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
        data: pubKeyBase,
        isEnabled: true,
      },
    ],
  });

  const identityPublicKey = new IdentityPublicKey()
    .setId(publicKeyId)
    .setType(IdentityPublicKey.TYPES.ECDSA_SECP256K1)
    .setData(pubKeyBase);

  identityCreateTransition.sign(identityPublicKey, privateKey);

  const validationResult = dpp.stateTransition.validateData(identityCreateTransition);
  if (!validationResult.isValid()) {
    throw new Error('Invalid identity create state transition');
  }

  await dapiClient.updateState(identityCreateTransition);

  await wait(2000); // wait a couple of seconds for tx to be confirmed

  return {
    identityId: identityCreateTransition.getIdentityId(),
    identityPrivateKey: privateKey,
  };
}

module.exports = registerUser;
