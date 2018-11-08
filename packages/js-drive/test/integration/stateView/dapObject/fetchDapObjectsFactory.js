const { mocha: { startMongoDb } } = require('@dashevo/js-evo-services-ctl');

const Reference = require('../../../../lib/stateView/Reference');
const DapObject = require('../../../../lib/stateView/dapObject/DapObject');
const DapObjectMongoDbRepository = require('../../../../lib/stateView/dapObject/DapObjectMongoDbRepository');
const createDapObjectMongoDbRepositoryFactory = require('../../../../lib/stateView/dapObject/createDapObjectMongoDbRepositoryFactory');
const fetchDapObjectsFactory = require('../../../../lib/stateView/dapObject/fetchDapObjectsFactory');

describe('fetchDapObjectsFactory', () => {
  const dapId = '9876';
  const objectData = {
    pver: 1,
    idx: 0,
    act: 0,
    objtype: 'DashPayContact',
    user: 'dashy',
    rev: 0,
  };
  const blockchainUserId = 'b8ae412cdeeb4bb39ec496dec34495ecccaf74f9fa9ea';
  const isDeleted = false;
  const blockHash = 'b8ae412cdeeb4bb39ec496dec34495ecccaf74f9fa9eaa712c77a03eb1994e75';
  const blockHeight = 1;
  const headerHash = '17jasdjk129uasd8asd023098SD09023jll123jlasd90823jklD';
  const hashSTPacket = 'ad877138as8012309asdkl123l123lka908013';
  const objectHash = '923jasd2a82j29fx0zx931ll21308sa';
  const reference = new Reference(
    blockHash,
    blockHeight,
    headerHash,
    hashSTPacket,
    objectHash,
  );
  const dapObject = new DapObject(blockchainUserId, objectData, reference, isDeleted);

  let createDapObjectMongoDbRepository;
  let fetchDapObjects;
  startMongoDb().then(async (mongoDbInstance) => {
    const mongoClient = await mongoDbInstance.getClient();
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
    expect(result).to.be.deep.equal([dapObject.getOriginalData()]);
  });

  it('should fetch DAP objects for specific DAP id, object type and user', async () => {
    const type = 'DashPayContact';
    const dapObjectRepository = createDapObjectMongoDbRepository(dapId);
    await dapObjectRepository.store(dapObject);
    const options = { where: { 'data.user': 'dashy' } };
    const result = await fetchDapObjects(dapId, type, options);
    expect(result).to.be.deep.equal([dapObject.getOriginalData()]);
  });

  it('should return empty array for specific DAP ID, object type and user not exist', async () => {
    const type = 'DashPayContact';
    const dapObjectRepository = createDapObjectMongoDbRepository(dapId);
    await dapObjectRepository.store(dapObject);
    const options = { where: { 'data.user': 'unknown' } };
    const result = await fetchDapObjects(dapId, type, options);
    expect(result).to.be.deep.equal([]);
  });

  it('should return empty array if DAP ID does not exist', async () => {
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
