const startMongoDbInstance = require('../../../../lib/test/services/mocha/startMongoDbInstance');

const Reference = require('../../../../lib/stateView/Reference');
const DapObject = require('../../../../lib/stateView/dapObject/DapObject');
const DapObjectMongoDbRepository = require('../../../../lib/stateView/dapObject/DapObjectMongoDbRepository');
const createDapObjectMongoDbRepositoryFactory = require('../../../../lib/stateView/dapObject/createDapObjectMongoDbRepositoryFactory');
const fetchDapObjectsFactory = require('../../../../lib/stateView/dapObject/fetchDapObjectsFactory');

describe('fetchDapObjectsFactory', () => {
  const dapId = '9876';
  const id = '123456';
  const objectData = {
    id,
    act: 0,
    objtype: 'DashPayContact',
    user: 'dashy',
    rev: 0,
  };
  const blockHash = 'b8ae412cdeeb4bb39ec496dec34495ecccaf74f9fa9eaa712c77a03eb1994e75';
  const blockHeight = 1;
  const headerHash = '17jasdjk129uasd8asd023098SD09023jll123jlasd90823jklD';
  const hashSTPacket = 'ad877138as8012309asdkl123l123lka908013';
  const reference = new Reference(
    blockHash,
    blockHeight,
    headerHash,
    hashSTPacket,
  );
  const dapObject = new DapObject(objectData, reference);

  let createDapObjectMongoDbRepository;
  let fetchDapObjects;
  startMongoDbInstance().then(async (mongoDbInstance) => {
    const mongoClient = await mongoDbInstance.mongoClient;
    createDapObjectMongoDbRepository = createDapObjectMongoDbRepositoryFactory(
      mongoClient,
      DapObjectMongoDbRepository,
    );
    fetchDapObjects = fetchDapObjectsFactory(createDapObjectMongoDbRepository);
  });

  it('should fetch DapObjects for specific DAP id and object type', async () => {
    const type = 'DashPayContact';
    const dapObjectRepository = createDapObjectMongoDbRepository(dapId);
    await dapObjectRepository.store(dapObject);
    const result = await fetchDapObjects(dapId, type);
    expect(result).to.be.deep.equal([dapObject]);
  });

  it('should fetch DapObjects for specific DAP id, object type and user', async () => {
    const type = 'DashPayContact';
    const dapObjectRepository = createDapObjectMongoDbRepository(dapId);
    await dapObjectRepository.store(dapObject);
    const options = { where: { 'object.user': 'dashy' } };
    const result = await fetchDapObjects(dapId, type, options);
    expect(result).to.be.deep.equal([dapObject]);
  });

  it('should return empty array for specific DAP id, object type and user not exist', async () => {
    const type = 'DashPayContact';
    const dapObjectRepository = createDapObjectMongoDbRepository(dapId);
    await dapObjectRepository.store(dapObject);
    const options = { where: { 'object.user': 'unknown' } };
    const result = await fetchDapObjects(dapId, type, options);
    expect(result).to.be.deep.equal([]);
  });

  it('should return empty array if DAP id does not exist', async () => {
    const unknowDapId = 'Unknown';
    const type = 'DashPayContact';
    const dapObjectRepository = createDapObjectMongoDbRepository(dapId);
    await dapObjectRepository.store(dapObject);
    const result = await fetchDapObjects(unknowDapId, type);
    expect(result).to.be.deep.equal([]);
  });

  it('should return empty array if type does not exist', async () => {
    const type = 'Unknown';
    const dapObjectRepository = createDapObjectMongoDbRepository(dapId);
    await dapObjectRepository.store(dapObject);
    const result = await fetchDapObjects(dapId, type);
    expect(result).to.be.deep.equal([]);
  });
});
