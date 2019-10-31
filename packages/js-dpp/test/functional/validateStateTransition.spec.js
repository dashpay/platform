const { startDrive } = require('@dashevo/dp-services-ctl');

const {
  StartTransactionRequest,
  ApplyStateTransitionRequest,
  CommitTransactionRequest,
} = require('@dashevo/drive-grpc');

const {
  Transaction,
  PrivateKey,
} = require('@dashevo/dashcore-lib');

const DataContractStateTransition = require(
  '../../lib/dataContract/stateTransition/DataContractStateTransition',
);
const DocumentsStateTransition = require(
  '../../lib/document/stateTransition/DocumentsStateTransition',
);

const getDataContractFixture = require('../../lib/test/fixtures/getDataContractFixture');
const getDocumentsFixture = require('../../lib/test/fixtures/getDocumentsFixture');

async function registerUser(coreApi) {
  await coreApi.generate(700);

  const { result: addressString } = await coreApi.getNewAddress();
  const { result: privateKeyString } = await coreApi.dumpPrivKey(addressString);

  const privateKey = new PrivateKey(privateKeyString);

  await coreApi.generate(500);
  await coreApi.sendToAddress(addressString, 10);
  await coreApi.generate(10);

  const { result: unspent } = await coreApi.listUnspent();
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

  const { result: userId } = await coreApi.sendrawtransaction(transaction.serialize());

  return userId;
}

describe('validateStateTransition', function main() {
  this.timeout(180000);

  let driveInstance;

  let driveApi;
  let driveUpdateStateApi;
  let coreApi;

  let dataContract;
  let documents;

  async function withinBlock(call) {
    const blockHeight = 0;
    const blockHash = Buffer.alloc(16, 0);

    const startRequest = new StartTransactionRequest();
    startRequest.setBlockHeight(blockHeight);

    await driveUpdateStateApi.startTransaction(startRequest);

    await call(
      blockHeight,
      blockHash,
    );

    const commitRequest = new CommitTransactionRequest();
    commitRequest.setBlockHeight(blockHeight);
    commitRequest.setBlockHash(blockHash);

    await driveUpdateStateApi.commitTransaction(commitRequest);
  }

  beforeEach(async () => {
    driveInstance = await startDrive();

    driveApi = driveInstance.driveApi.getApi();
    driveUpdateStateApi = driveInstance.driveUpdateState.getApi();
    coreApi = driveInstance.dashCore.getApi();

    dataContract = getDataContractFixture();
    documents = getDocumentsFixture();

    const userId = await registerUser(coreApi);

    dataContract.contractId = userId;

    documents.forEach((d) => {
      // eslint-disable-next-line
      d.contractId = userId;
      // eslint-disable-next-line
      d.userId = userId;
    });
  });

  it('should validate contract state transition without a blockchain user', async () => {
    dataContract.contractId = Buffer.alloc(32).toString('hex');

    const stateTransition = new DataContractStateTransition(dataContract);

    await withinBlock(async (blockHeight, blockHash) => {
      const request = new ApplyStateTransitionRequest();
      request.setStateTransition(stateTransition.serialize());
      request.setBlockHeight(blockHeight);
      request.setBlockHash(blockHash);

      try {
        await driveUpdateStateApi.applyStateTransition(request);
        expect.fail('Error was not thrown');
      } catch (e) {
        const [error] = JSON.parse(e.metadata.get('errors')[0]);
        expect(error.message).to.equal('User not found');
        expect(e.message).to.equal('3 INVALID_ARGUMENT: Invalid argument: Invalid State Transition');
      }
    });
  });

  it('should validate contract state transition when it submitted twice', async () => {
    const stateTransition = new DataContractStateTransition(dataContract);

    await withinBlock(async (blockHeight, blockHash) => {
      const request = new ApplyStateTransitionRequest();
      request.setStateTransition(stateTransition.serialize());
      request.setBlockHeight(blockHeight);
      request.setBlockHash(blockHash);

      await driveUpdateStateApi.applyStateTransition(request);
    });

    await withinBlock(async (blockHeight, blockHash) => {
      const request = new ApplyStateTransitionRequest();
      request.setStateTransition(stateTransition.serialize());
      request.setBlockHeight(blockHeight);
      request.setBlockHash(blockHash);

      try {
        await driveUpdateStateApi.applyStateTransition(request);
      } catch (e) {
        const [error] = JSON.parse(e.metadata.get('errors')[0]);
        expect(error.message).to.equal('Data Contract is already present');
      }
    });
  });

  it('should validate document uniqueness by using indicies', async () => {
    const [,,, indexDocument, anotherDocument] = documents;

    anotherDocument.set('lastName', 'Birkin');

    const stateTransition = new DataContractStateTransition(dataContract);

    await withinBlock(async (blockHeight, blockHash) => {
      const request = new ApplyStateTransitionRequest();
      request.setStateTransition(stateTransition.serialize());
      request.setBlockHeight(blockHeight);
      request.setBlockHash(blockHash);

      await driveUpdateStateApi.applyStateTransition(request);
    });

    const documentsStateTransition = new DocumentsStateTransition([indexDocument]);

    const duplicateStateTransition = new DocumentsStateTransition([anotherDocument]);

    await withinBlock(async (blockHeight, blockHash) => {
      const request = new ApplyStateTransitionRequest();
      request.setStateTransition(documentsStateTransition.serialize());
      request.setBlockHeight(blockHeight);
      request.setBlockHash(blockHash);

      await driveUpdateStateApi.applyStateTransition(request);

      const anotherRequest = new ApplyStateTransitionRequest();
      anotherRequest.setStateTransition(duplicateStateTransition.serialize());
      anotherRequest.setBlockHeight(blockHeight + 1);
      anotherRequest.setBlockHash(Buffer.alloc(32, blockHeight + 1));

      try {
        await driveUpdateStateApi.applyStateTransition(anotherRequest);
      } catch (e) {
        const [error] = JSON.parse(e.metadata.get('errors')[0]);
        expect(error.message).to.equal('Duplicate Document found');
        expect(error.document).to.deep.equal(anotherDocument.toJSON());
        expect(error.indexDefinition).to.deep.equal({
          unique: true,
          properties: [
            { $userId: 'asc' },
            { lastName: 'desc' },
          ],
        });
      }
    });
  });

  it('should successfuly submit valid contract and documents', async () => {
    const stateTransition = new DataContractStateTransition(dataContract);

    await withinBlock(async (blockHeight, blockHash) => {
      const request = new ApplyStateTransitionRequest();
      request.setStateTransition(stateTransition.serialize());
      request.setBlockHeight(blockHeight);
      request.setBlockHash(blockHash);

      await driveUpdateStateApi.applyStateTransition(request);
    });

    const documentsStateTransition = new DocumentsStateTransition(documents);

    await withinBlock(async (blockHeight, blockHash) => {
      const request = new ApplyStateTransitionRequest();
      request.setStateTransition(documentsStateTransition.serialize());
      request.setBlockHeight(blockHeight);
      request.setBlockHash(blockHash);

      await driveUpdateStateApi.applyStateTransition(request);
    });

    const { result: contract } = await driveApi.request('fetchContract', {
      contractId: dataContract.getId(),
    });

    expect(contract).to.deep.equal(dataContract.toJSON());

    const { result: niceDocuments } = await driveApi.request('fetchDocuments', {
      contractId: dataContract.getId(),
      type: 'niceDocument',
    });

    const { result: prettyDocuments } = await driveApi.request('fetchDocuments', {
      contractId: dataContract.getId(),
      type: 'prettyDocument',
    });

    const { result: indexedDocuments } = await driveApi.request('fetchDocuments', {
      contractId: dataContract.getId(),
      type: 'indexedDocument',
    });

    expect(
      niceDocuments
        .concat(prettyDocuments)
        .concat(indexedDocuments),
    ).to.have.deep.members(
      documents
        .map(d => d.toJSON()),
    );
  });

  afterEach(async () => {
    await driveInstance.remove();
  });
});
