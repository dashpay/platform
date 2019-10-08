const {
  StartTransactionRequest,
  ApplyStateTransitionRequest,
  CommitTransactionRequest,
  StartTransactionResponse,
  ApplyStateTransitionResponse,
  CommitTransactionResponse,
} = require('@dashevo/drive-grpc');
const {
  startDrive,
} = require('@dashevo/dp-services-ctl');

const getSTPacketsFixture = require('../../lib/test/fixtures/getSTPacketsFixture');
const createStateTransition = require('../../lib/test/createStateTransition');
const registerUser = require('../../lib/test/registerUser');

describe('updateState', function main() {
  let grpcClient;
  let driveApiClient;
  let stPacket;
  let stateTransition;
  let startTransactionRequest;
  let applyStateTransitionRequest;
  let commitTransactionRequest;
  let driveInstance;

  const height = 1;
  const hash = 'b4749f017444b051c44dfd2720e88f314ff94f3dd6d56d40ef65854fcd7fff6b';

  this.timeout(90000);

  beforeEach(async () => {
    driveInstance = await startDrive();

    grpcClient = driveInstance.driveUpdateState.getApi();
    driveApiClient = driveInstance.driveApi.getApi();
    const coreApi = driveInstance.dashCore.getApi();

    // activate sporks
    await coreApi.generate(1000);

    const { userId, privateKeyString: userPrivateKeyString } = await registerUser('testUser', coreApi);

    [stPacket] = getSTPacketsFixture();
    stateTransition = createStateTransition(userId, userPrivateKeyString, stPacket);

    startTransactionRequest = new StartTransactionRequest();
    startTransactionRequest.setBlockHeight(height);

    applyStateTransitionRequest = new ApplyStateTransitionRequest();
    applyStateTransitionRequest.setStateTransitionHeader(Buffer.from(stateTransition.serialize(), 'hex'));
    applyStateTransitionRequest.setStateTransitionPacket(stPacket.serialize());
    applyStateTransitionRequest.setBlockHeight(height);
    applyStateTransitionRequest.setBlockHash(hash);

    commitTransactionRequest = new CommitTransactionRequest();
    commitTransactionRequest.setBlockHeight();
  });

  after(async () => {
    await Promise.all([
      driveInstance.remove(),
    ]);
  });

  it('should successfully apply state transition and commit data', async () => {
    const startTransactionResponse = await grpcClient.startTransaction(startTransactionRequest);
    const applyStateTransitionResponse = await grpcClient
      .applyStateTransition(applyStateTransitionRequest);
    const commitTransactionResponse = await grpcClient.commitTransaction(commitTransactionRequest);

    expect(startTransactionResponse).to.be.an.instanceOf(StartTransactionResponse);
    expect(applyStateTransitionResponse).to.be.an.instanceOf(ApplyStateTransitionResponse);
    expect(commitTransactionResponse).to.be.an.instanceOf(CommitTransactionResponse);

    // check we have our data in database
    const contract = await driveApiClient.request('fetchContract', { contractId: stPacket.getContractId() });
    const documents = await Promise.all(
      stPacket.getDocuments().map(
        document => driveApiClient.fetchDocuments(stPacket.getContractId(), document.getType()),
      ),
    );

    expect(contract.result).to.deep.equal(stPacket.getContract().toJSON());

    const storedDocumentsJson = documents.map(document => document.toJSON());
    const stPacketDocumentsJson = stPacket.getDocuments().map(document => document.toJSON());

    expect(storedDocumentsJson).to.deep.equal(stPacketDocumentsJson);
  });
});
