const DapObject = require('../../../../lib/stateView/dapObject/DapObject');
const Reference = require('../../../../lib/stateView/Reference');
const createDapObjectMongoDbRepositoryFactory = require('../../../../lib/stateView/dapObject/createDapObjectMongoDbRepositoryFactory');
const DapObjectMongoDbRepository = require('../../../../lib/stateView/dapObject/DapObjectMongoDbRepository');
const { mocha: { startMongoDb } } = require('js-evo-services-ctl');

describe('createDapObjectMongoDbRepositoryFactory', () => {
  let createDapObjectMongoDbRepository;
  startMongoDb().then(async (mongoDbInstance) => {
    const mongoClient = await mongoDbInstance.mongoClient;
    createDapObjectMongoDbRepository = createDapObjectMongoDbRepositoryFactory(
      mongoClient,
      DapObjectMongoDbRepository,
    );
  });

  it('should create DapObjectMongoDbRepository', async () => {
    const dapId = 'ac5784e7dd8fc9f1b638a353fb10015d3841bb9076c20e2ebefc3e97599e92b5';
    const dapObjectRepository = createDapObjectMongoDbRepository(dapId);

    const blockchainUserId = '3557b9a8dfcc1ef9674b50d8d232e0e3e9020f49fa44f89cace622a01f43d03e';
    const objectData = {
      idx: 0,
    };
    const reference = new Reference();
    const dapObject = new DapObject(blockchainUserId, objectData, reference);
    await dapObjectRepository.store(dapObject);

    const result = await dapObjectRepository.find(dapObject.getId());
    expect(result.toJSON().blockchainUserId).to.equal(blockchainUserId);
  });
});
