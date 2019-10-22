const {
  startDapi,
} = require('@dashevo/dp-services-ctl');

const {
  UpdateStateTransitionResponse,
} = require('@dashevo/dapi-grpc');

const {
  PrivateKey,
  Transaction,
} = require('@dashevo/dashcore-lib');

const getStPacketFixture = require('../../../../../lib/test/fixtures/getStPacketFixture');
const createStateTransition = require('../../../../../lib/test/createStateTransition');

describe('updateStateHandlerFactory', function main() {
  this.timeout(160000);

  let removeDapi;
  let dapiClient;
  let coreAPI;
  let driveClient;
  let stHeader;
  let stPacket;
  let stPacketFixture;
  let userId;

  beforeEach(async () => {
    const {
      driveApi,
      dapiCore,
      dashCore,
      remove,
    } = await startDapi();

    removeDapi = remove;

    dapiClient = dapiCore.getApi();
    driveClient = driveApi.getApi();
    coreAPI = dashCore.getApi();

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

    ({ result: userId } = await coreAPI.sendrawtransaction(transaction.serialize()));

    await coreAPI.generate(1);

    stPacketFixture = getStPacketFixture();

    const stateTransition = createStateTransition(userId, privateKeyString, stPacketFixture);

    stHeader = Buffer.from(stateTransition.serialize(), 'hex');
    stPacket = stPacketFixture.serialize();
  });

  afterEach(async () => {
    await removeDapi();
  });

  it('should respond with valid result and store contract', async () => {
    const result = await dapiClient.updateState(stHeader, stPacket);
    const contractId = stPacketFixture.getContractId();
    const { result: contract } = await driveClient.request('fetchContract', { contractId });

    expect(result).to.be.an.instanceOf(UpdateStateTransitionResponse);
    expect(contract).to.deep.include(stPacketFixture.getContract().toJSON());
  });
});
