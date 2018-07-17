const createDapObjectMongoDbRepositoryFactory = require('../../../lib/stateView/dapObject/createDapObjectMongoDbRepositoryFactory');
const DapObjectMongoDbRepository = require('../../../lib/stateView/dapObject/DapObjectMongoDbRepository');
const DapContractMongoDbRepository = require('../../../lib/stateView/dapContract/DapContractMongoDbRepository');
const startMongoDbInstance = require('../../../lib/test/services/mocha/startMongoDbInstance');
const startIPFSInstance = require('../../../lib/test/services/mocha/startIPFSInstance');
const hashSTPacket = require('../../../lib/test/consensus/hashSTPacket');
const updateDapContractFactory = require('../../../lib/stateView/dapContract/updateDapContractFactory');
const updateDapObjectFactory = require('../../../lib/stateView/dapObject/updateDapObjectFactory');
const applyStateTransitionFactory = require('../../../lib/stateView/applyStateTransitionFactory');
const sanitizeData = require('../../../lib/mongoDb/sanitizeData');

const getBlockFixtures = require('../../../lib/test/fixtures/getBlockFixtures');
const getTransitionPacketFixtures = require('../../../lib/test/fixtures/getTransitionPacketFixtures');
const getTransitionHeaderFixtures = require('../../../lib/test/fixtures/getTransitionHeaderFixtures');

describe('applyStateTransitionFactory', () => {
  let mongoClient;
  let mongoDb;
  let ipfsClient;

  startMongoDbInstance().then(async (mongoDbInstance) => {
    ({ mongoClient } = mongoDbInstance);
    mongoDb = await mongoDbInstance.getMongoClient();
  });
  startIPFSInstance().then(async (ipfsInstance) => {
    ipfsClient = await ipfsInstance.getApi();
  });

  it('should compute DapContract state view', async () => {
    const block = getBlockFixtures()[0];
    const packet = getTransitionPacketFixtures()[0];
    const header = getTransitionHeaderFixtures()[0].toJSON();
    header.hashSTPacket = await hashSTPacket(packet);

    await ipfsClient.dag.put(packet, {
      format: 'dag-cbor',
      hashAlg: 'sha2-256',
    });

    const dapContractMongoDbRepository = new DapContractMongoDbRepository(mongoDb, sanitizeData);
    const createDapObjectMongoDbRepository = createDapObjectMongoDbRepositoryFactory(
      mongoClient,
      DapObjectMongoDbRepository,
    );
    const updateDapContract = updateDapContractFactory(dapContractMongoDbRepository);
    const updateDapObject = updateDapObjectFactory(createDapObjectMongoDbRepository);
    const applyStateTransition = applyStateTransitionFactory(
      ipfsClient,
      updateDapContract,
      updateDapObject,
    );
    await applyStateTransition(header, block);
  });

  it('should compute DapObject state view', async () => {
    const block = getBlockFixtures()[1];
    const packet = getTransitionPacketFixtures()[1];
    const header = getTransitionHeaderFixtures()[1].toJSON();
    header.hashSTPacket = await hashSTPacket(packet);

    await ipfsClient.dag.put(packet, {
      format: 'dag-cbor',
      hashAlg: 'sha2-256',
    });

    const dapContractMongoDbRepository = new DapContractMongoDbRepository(mongoDb, sanitizeData);
    const createDapObjectMongoDbRepository = createDapObjectMongoDbRepositoryFactory(
      mongoClient,
      DapObjectMongoDbRepository,
    );
    const updateDapContract = updateDapContractFactory(dapContractMongoDbRepository);
    const updateDapObject = updateDapObjectFactory(createDapObjectMongoDbRepository);
    const applyStateTransition = applyStateTransitionFactory(
      ipfsClient,
      updateDapContract,
      updateDapObject,
    );
    await applyStateTransition(header, block);
  });
});
