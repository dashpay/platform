const {
  mocha: {
    startMongoDb,
    startIPFS,
  },
} = require('@dashevo/js-evo-services-ctl');

const Reference = require('../../../../lib/stateView/Reference');
const DapObjectMongoDbRepository = require('../../../../lib/stateView/dapObject/DapObjectMongoDbRepository');
const DapObject = require('../../../../lib/stateView/dapObject/DapObject');

const revertDapObjectsForStateTransitionFactory = require('../../../../lib/stateView/dapObject/revertDapObjectsForStateTransitionFactory');
const createDapObjectMongoDbRepositoryFactory = require('../../../../lib/stateView/dapObject/createDapObjectMongoDbRepositoryFactory');
const updateDapObjectFactory = require('../../../../lib/stateView/dapObject/updateDapObjectFactory');
const applyStateTransitionFactory = require('../../../../lib/stateView/applyStateTransitionFactory');
const applyStateTransitionFromReferenceFactory = require('../../../../lib/stateView/applyStateTransitionFromReferenceFactory');

const addSTPacketFactory = require('../../../../lib/storage/ipfs/addSTPacketFactory');

const getBlockFixtures = require('../../../../lib/test/fixtures/getBlockFixtures');
const getHeaderFixtures = require('../../../../lib/test/fixtures/getTransitionHeaderFixtures');
const getPacketFixtures = require('../../../../lib/test/fixtures/getTransitionPacketFixtures');

const RpcClientMock = require('../../../../lib/test/mock/RpcClientMock');

describe('revertDapObjectsForStateTransitionFactory', () => {
  const blockchainUserId = '3557b9a8dfcc1ef9674b50d8d232e0e3e9020f49fa44f89cace622a01f43d03e';

  let mongoClient;
  startMongoDb().then(async (mongoDbInstance) => {
    mongoClient = await mongoDbInstance.getClient();
  });

  let ipfsAPI;
  startIPFS().then(async (ipfsInstance) => {
    ipfsAPI = await ipfsInstance.getApi();
  });

  let addSTPacket;
  let createDapObjectMongoDbRepository;
  let updateDapObject;
  let applyStateTransition;
  let rpcClientMock;
  let revertDapObjectsForStateTransition;
  beforeEach(function beforeEach() {
    addSTPacket = addSTPacketFactory(ipfsAPI);
    createDapObjectMongoDbRepository = createDapObjectMongoDbRepositoryFactory(
      mongoClient,
      DapObjectMongoDbRepository,
    );
    updateDapObject = updateDapObjectFactory(createDapObjectMongoDbRepository);
    applyStateTransition = applyStateTransitionFactory(
      ipfsAPI,
      null,
      updateDapObject,
      1000,
    );
    rpcClientMock = new RpcClientMock(this.sinon);
    const applyStateTransitionFromReference = applyStateTransitionFromReferenceFactory(
      applyStateTransition,
      rpcClientMock,
    );
    revertDapObjectsForStateTransition = revertDapObjectsForStateTransitionFactory(
      ipfsAPI,
      rpcClientMock,
      createDapObjectMongoDbRepository,
      applyStateTransition,
      applyStateTransitionFromReference,
      30 * 1000,
    );
  });

  it('should mark Dap Objects as deleted if there is no previous version', async () => {
    const [block] = getBlockFixtures();
    const packet = getPacketFixtures()[1];
    const [transition] = getHeaderFixtures();
    transition.extraPayload.hashSTPacket = packet.getHash();

    await addSTPacket(packet);

    const dapObjectRepository = createDapObjectMongoDbRepository(
      packet.dapid,
    );

    const dapObjectData = {
      objtype: 'user',
      idx: 0,
      rev: 1,
      act: 0,
    };

    const reference = new Reference(
      block.hash,
      block.height,
      transition.hash,
      packet.getHash(),
      null,
    );

    await updateDapObject(packet.dapid, blockchainUserId, reference, dapObjectData);

    await revertDapObjectsForStateTransition({
      stateTransition: transition,
    });

    const dapObjectList = await dapObjectRepository.fetch('user');

    expect(dapObjectList).to.be.empty();
  });

  it('should revert Dap Object to its previous revision if any', async () => {
    const blocks = getBlockFixtures();
    const transitions = getHeaderFixtures();
    const packet = getPacketFixtures()[1];

    const dapObjectRepository = createDapObjectMongoDbRepository(
      packet.dapid,
    );

    const references = [];

    let lastTransition;
    for (let i = 0; i < 3; i++) {
      const block = blocks[i];
      const transition = transitions[i];

      const [dapObjectData] = packet.dapobjects;
      dapObjectData.act = (i === 0 ? DapObject.ACTION_CREATE : DapObject.ACTION_UPDATE);
      dapObjectData.rev = i + 1;

      transition.extraPayload.regTxId = blockchainUserId;
      transition.extraPayload.hashSTPacket = packet.getHash();

      rpcClientMock.getRawTransaction
        .withArgs(transition.hash)
        .resolves({
          result: transition,
        });

      const reference = new Reference(
        block.hash,
        block.height,
        transition.hash,
        packet.getHash(),
        null,
      );

      references.push(reference);

      await addSTPacket(packet);
      await updateDapObject(packet.dapid, blockchainUserId, reference, dapObjectData);

      lastTransition = transition;
    }

    await revertDapObjectsForStateTransition({
      stateTransition: lastTransition,
    });

    const dapObjectList = await dapObjectRepository.fetch('user');

    expect(dapObjectList.length).to.be.equal(1);

    const [dapObject] = dapObjectList;

    expect(dapObject.revision).to.be.equal(2);
    expect(dapObject.reference).to.be.deep.equal(references[1]);

    const [previousRevision] = dapObject.getPreviousRevisions();

    expect(previousRevision.revision).to.be.equal(1);
    expect(previousRevision.reference).to.be.deep.equal(references[0]);
  });

  it('should not do anything if packet have no Dap ID');
});
