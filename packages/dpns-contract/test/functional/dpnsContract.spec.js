const { startDapi } = require('@dashevo/dp-services-ctl');

const DashPlatformPrtocol = require('@dashevo/dpp');

const {
  Transaction,
  PrivateKey,
} = require('@dashevo/dashcore-lib');

const dpnsDocumentsSchema = require('../../src/schema/dpns-documents.json');

/**
 * Register new blockchain user
 *
 * @param {CoreRPCClient} coreApi
 */
async function registerUser(coreAPI) {
  const { result: addressString } = await coreAPI.getNewAddress();
  const { result: privateKeyString } = await coreAPI.dumpPrivKey(addressString);

  const privateKey = new PrivateKey(privateKeyString);

  await coreAPI.generate(500);
  await coreAPI.sendToAddress(addressString, 10);
  await coreAPI.generate(10);

  const { result: unspent } = await coreAPI.listUnspent();
  const inputs = unspent.filter(input => input.address === addressString);

  const transactionPayload = new Transaction.Payload.SubTxRegisterPayload();

  const userName = 'dashUser';

  transactionPayload.setUserName(userName)
    .setPubKeyIdFromPrivateKey(privateKey)
    .sign(privateKey);

  const transaction = new Transaction({
    type: Transaction.TYPES.TRANSACTION_SUBTX_REGISTER,
    version: 3,
    extraPayload: transactionPayload.toString(),
  });

  transaction.from(inputs)
    .addFundingOutput(10000)
    .change(addressString)
    .fee(668)
    .sign(privateKey);

  const { result: userId } = await coreAPI.sendrawtransaction(transaction.serialize());

  await coreAPI.generate(1);

  return { userId, privateKey };
}

describe('DPNS', function describe() {
  this.timeout(160000);

  let dpp;
  let contract;
  let userId;
  let privateKey;

  let dapiClient;
  let removeDapi;

  beforeEach(async () => {
    dpp = new DashPlatformPrtocol();

    // Create the contract
    contract = dpp.contract.create('DPNSContract', dpnsDocumentsSchema);
    dpp.setContract(contract);

    const {
      dashCore,
      dapiCore,
      remove,
    } = await startDapi();

    dapiClient = dapiCore.getApi();
    removeDapi = remove;

    // Register blockchain user
    ({ userId, privateKey } = await registerUser(dashCore.getApi()));
    dpp.setUserId(userId);

    // Submit the contract
    const contractPacket = dpp.packet.create(contract);

    const transaction = new Transaction()
      .setType(Transaction.TYPES.TRANSACTION_SUBTX_TRANSITION);

    transaction.extraPayload
      .setRegTxId(userId)
      .setHashPrevSubTx(userId)
      .setHashSTPacket(contractPacket.hash())
      .setCreditFee(1000)
      .sign(privateKey);

    await dapiClient.sendRawTransition(
      transaction.serialize(),
      contractPacket.serialize().toString('hex'),
    );
  });

  afterEach(async () => {
    await removeDapi();
  });

  it('should throw an error if preorder was not submitted before registering a domain');

  it('should thow an error if parent domain is not found');

  it('should successfuly submit preorder and domain documents');

  it('shoud not allow domain update');

  it('should not allow damain delete');
});
