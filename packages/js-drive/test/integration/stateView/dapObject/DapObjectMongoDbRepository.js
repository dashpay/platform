const DapObject = require('../../../../lib/stateView/dapObject/DapObject');
const DapObjectMongoDbRepository = require('../../../../lib/stateView/dapObject/DapObjectMongoDbRepository');
const startMongoDbInstance = require('../../../../lib/test/services/mocha/startMongoDbInstance');

describe('DapObjectMongoDbRepository', () => {
  let dapObjectRepository;
  startMongoDbInstance().then(async (mongoDbInstance) => {
    const mongoClient = await mongoDbInstance.mongoClient;
    const mongoDb = mongoClient.db('test_dap');
    dapObjectRepository = new DapObjectMongoDbRepository(mongoDb);
  });

  it('should store DapObject entity', async () => {
    const id = '123456';
    const dapObject = new DapObject(id);

    await dapObjectRepository.store(dapObject);
    const object = await dapObjectRepository.find(id);
    expect(object.toJSON()).to.deep.equal(dapObject.toJSON());
  });

  it('should return empty DapObject if not found', async () => {
    const object = await dapObjectRepository.find();

    const serializeObject = object.toJSON();
    expect(serializeObject.id).to.not.exist();
  });
});
