const {
  mocha: {
    startMongoDb,
    startIPFS,
  },
} = require('@dashevo/js-evo-services-ctl');
const Reference = require('../../../lib/stateView/Reference');
const DapContract = require('../../../lib/stateView/dapContract/DapContract');
const createDapObjectMongoDbRepositoryFactory = require('../../../lib/stateView/dapObject/createDapObjectMongoDbRepositoryFactory');
const DapObjectMongoDbRepository = require('../../../lib/stateView/dapObject/DapObjectMongoDbRepository');
const DapContractMongoDbRepository = require('../../../lib/stateView/dapContract/DapContractMongoDbRepository');
const updateDapContractFactory = require('../../../lib/stateView/dapContract/updateDapContractFactory');
const updateDapObjectFactory = require('../../../lib/stateView/dapObject/updateDapObjectFactory');
const applyStateTransitionFactory = require('../../../lib/stateView/applyStateTransitionFactory');
const serializer = require('../../../lib/util/serializer');

const getBlockFixtures = require('../../../lib/test/fixtures/getBlockFixtures');
const getTransitionPacketFixtures = require('../../../lib/test/fixtures/getTransitionPacketFixtures');
const getTransitionHeaderFixtures = require('../../../lib/test/fixtures/getTransitionHeaderFixtures');
const addSTPacketFactory = require('../../../lib/storage/ipfs/addSTPacketFactory');
const generateDapObjectId = require('../../../lib/stateView/dapObject/generateDapObjectId');

const doubleSha256 = require('../../../lib/util/doubleSha256');

describe('applyStateTransitionFactory', () => {
  let mongoClient;
  let mongoDb;
  let ipfsClient;

  startMongoDb().then(async (mongoDbInstance) => {
    mongoClient = await mongoDbInstance.getClient();
    mongoDb = await mongoDbInstance.getDb();
  });
  startIPFS().then(async (ipfsInstance) => {
    ipfsClient = await ipfsInstance.getApi();
  });

  let addSTPacket;
  let dapContractMongoDbRepository;
  let createDapObjectMongoDbRepository;
  let applyStateTransition;
  beforeEach(() => {
    addSTPacket = addSTPacketFactory(ipfsClient);
    dapContractMongoDbRepository = new DapContractMongoDbRepository(mongoDb, serializer);
    createDapObjectMongoDbRepository = createDapObjectMongoDbRepositoryFactory(
      mongoClient,
      DapObjectMongoDbRepository,
    );
    const updateDapContract = updateDapContractFactory(dapContractMongoDbRepository);
    const updateDapObject = updateDapObjectFactory(createDapObjectMongoDbRepository);
    applyStateTransition = applyStateTransitionFactory(
      ipfsClient,
      updateDapContract,
      updateDapObject,
      1000,
    );
  });

  it('should compute DapContract state view', async () => {
    const block = getBlockFixtures()[0];
    const packet = getTransitionPacketFixtures()[0];
    const header = getTransitionHeaderFixtures()[0];
    header.extraPayload.hashSTPacket = packet.getHash();

    await addSTPacket(packet);
    await applyStateTransition(header, block);

    const dapId = doubleSha256(packet.dapcontract);
    const dapContract = await dapContractMongoDbRepository.find(dapId);

    expect(dapContract.getDapId()).to.be.equal(dapId);
    expect(dapContract.getDapName()).to.be.equal(packet.dapcontract.dapname);
    expect(dapContract.getOriginalData()).to.be.deep.equal(packet.dapcontract);
    expect(dapContract.getVersion()).to.be.deep.equal(packet.dapcontract.dapver);
    expect(dapContract.getPreviousVersions()).to.be.deep.equal([]);
  });

  it('should update with revisions DapContract state view', async () => {
    const dapId = '1234';
    const dapName = 'DashPay';

    const firstReference = new Reference(null, null, null, null, null);
    const firstData = {
      dapname: dapName,
      dapver: 1,
    };
    const firstVersionDeleted = false;
    const firstPreviousVersions = [];
    const firstDapContractVersion = new DapContract(
      dapId,
      firstData,
      firstReference,
      firstVersionDeleted,
      firstPreviousVersions,
    );
    await dapContractMongoDbRepository.store(firstDapContractVersion);

    const block = getBlockFixtures()[0];
    const packet = getTransitionPacketFixtures()[0];
    packet.dapcontract.dapver = 2;
    packet.dapcontract.upgradedapid = dapId;
    const header = getTransitionHeaderFixtures()[0];
    header.extraPayload.hashSTPacket = packet.getHash();

    await addSTPacket(packet);
    await applyStateTransition(header, block);

    const dapContract = await dapContractMongoDbRepository.find(dapId);

    expect(dapContract.getDapId()).to.be.equal(dapId);
    expect(dapContract.getDapName()).to.be.equal(packet.dapcontract.dapname);
    expect(dapContract.getOriginalData()).to.be.deep.equal(packet.dapcontract);
    expect(dapContract.getVersion()).to.be.deep.equal(packet.dapcontract.dapver);
    expect(dapContract.getPreviousVersions()).to.be.deep.equal([
      firstDapContractVersion.currentRevision(),
    ]);
  });

  it('should compute DapObject state view', async () => {
    const block = getBlockFixtures()[1];
    const packet = getTransitionPacketFixtures()[1];
    packet.dapobjects[0].act = 0;
    const header = getTransitionHeaderFixtures()[1];
    header.extraPayload.hashSTPacket = packet.getHash();

    await addSTPacket(packet);
    await applyStateTransition(header, block);

    const dapId = packet.dapid;
    const dapObjectRepository = createDapObjectMongoDbRepository(dapId);
    const objectType = packet.dapobjects[0].objtype;
    const objects = await dapObjectRepository.fetch(objectType);

    expect(objects.length).to.be.equal(1);

    const dapObject = objects[0];
    const blockchainUserId = header.extraPayload.regTxId;
    const slotNumber = packet.dapobjects[0].idx;
    const dapObjectId = generateDapObjectId(blockchainUserId, slotNumber);

    expect(dapObject.getId()).to.be.equal(dapObjectId);
  });
});
