const {
  startDapi,
} = require('@dashevo/dp-services-ctl');

const {
  PrivateKey,
  Transaction,
} = require('@dashevo/dashcore-lib');

describe('getLastUserStateTransitionHashHandlerFactory', function main() {
  this.timeout(160000);

  let coreAPI;
  let dapiClient;
  let removeDapi;

  let userId;
  let lastStateTransitionHash;

  beforeEach(async () => {
    const {
      dashCore,
      dapiCore,
      remove,
    } = await startDapi();

    removeDapi = remove;

    coreAPI = dashCore.getApi();
    dapiClient = dapiCore.getApi();

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

    lastStateTransitionHash = transaction.hash;

    ({ result: userId } = await coreAPI.sendrawtransaction(transaction.serialize()));

    await coreAPI.generate(1);
  });

  afterEach(async () => {
    await removeDapi();
  });

  it('should respond error if userId is not provided or empty', async () => {
    try {
      await dapiClient.getLastUserStateTransitionHash('');

      expect.fail('Error was not thrown');
    } catch (e) {
      expect(e.message).to.equal('3 INVALID_ARGUMENT: Invalid argument: userId is not specified');
    }
  });

  it('should respond error if user was not found', async () => {
    try {
      userId = Buffer.alloc(256, 1).toString('hex');

      await dapiClient.getLastUserStateTransitionHash(userId);

      expect.fail('Error was not thrown');
    } catch (e) {
      expect(e.message).to.equal(
        `3 INVALID_ARGUMENT: Invalid argument: Could not retrieve user by id ${userId}. Reason: user ${userId} not found`,
      );
    }
  });

  it('should respond with last state transition hash for a user', async () => {
    const result = await dapiClient.getLastUserStateTransitionHash(userId);

    expect(result).to.equal(lastStateTransitionHash);
  });
});
