const { mocha: { startMongoDb } } = require('@dashevo/js-evo-services-ctl');

const SVObjectMongoDbRepository = require('../../../../lib/stateView/object/SVObjectMongoDbRepository');

const sanitizer = require('../../../../lib/mongoDb/sanitizer');
const createSVObjectMongoDbRepositoryFactory = require('../../../../lib/stateView/object/createSVObjectMongoDbRepositoryFactory');
const fetchDPObjectsFactory = require('../../../../lib/stateView/object/fetchDPObjectsFactory');

const getSVObjectsFixture = require('../../../../lib/test/fixtures/getSVObjectsFixture');

describe('fetchDPObjectsFactory', () => {
  let createSVObjectMongoDbRepository;
  let fetchDPObjects;
  let mongoClient;
  let svObject;
  let type;
  let contractId;
  let dpObject;

  startMongoDb().then((mongoDb) => {
    mongoClient = mongoDb.getClient();
  });

  beforeEach(() => {
    createSVObjectMongoDbRepository = createSVObjectMongoDbRepositoryFactory(
      mongoClient,
      SVObjectMongoDbRepository,
      sanitizer,
    );

    fetchDPObjects = fetchDPObjectsFactory(createSVObjectMongoDbRepository);

    [svObject] = getSVObjectsFixture();

    dpObject = svObject.getDPObject();
    type = dpObject.getType();
    contractId = 'b8ae412cdeeb4bb39ec496dec34495ecccaf74f9fa9eaa712c77a03eb1994e75';
  });

  it('should fetch DP Objects for specified contract ID and object type', async () => {
    const svObjectRepository = createSVObjectMongoDbRepository(contractId, type);
    await svObjectRepository.store(svObject);

    const result = await fetchDPObjects(contractId, type);

    expect(result).to.be.a('array');
    expect(result).to.have.lengthOf(1);

    const [actualDPObject] = result;

    expect(actualDPObject.toJSON()).to.be.deep.equal(dpObject.toJSON());
  });

  it('should fetch DP objects for specified contract id, object type and name', async () => {
    let result = await fetchDPObjects(contractId, type);

    expect(result).to.be.deep.equal([]);

    const svObjectRepository = createSVObjectMongoDbRepository(contractId, type);
    await svObjectRepository.store(svObject);

    const options = { where: { 'dpObject.name': dpObject.get('name') } };
    result = await fetchDPObjects(contractId, type, options);

    expect(result).to.be.a('array');
    expect(result).to.have.lengthOf(1);

    const [actualDPObject] = result;

    expect(actualDPObject.toJSON()).to.be.deep.equal(dpObject.toJSON());
  });

  it('should return empty array for specified contract ID, object type and name not exist', async () => {
    const svObjectRepository = createSVObjectMongoDbRepository(contractId, type);
    await svObjectRepository.store(svObject);

    const options = { where: { 'dpObject.name': 'unknown' } };

    const result = await fetchDPObjects(contractId, type, options);

    expect(result).to.be.deep.equal([]);
  });

  it('should return empty array if contract ID does not exist', async () => {
    const svObjectRepository = createSVObjectMongoDbRepository(contractId, type);

    await svObjectRepository.store(svObject);

    contractId = 'Unknown';

    const result = await fetchDPObjects(contractId, type);

    expect(result).to.be.deep.equal([]);
  });

  it('should return empty array if type does not exist', async () => {
    const svObjectRepository = createSVObjectMongoDbRepository(contractId, type);

    await svObjectRepository.store(svObject);

    type = 'Unknown';

    const result = await fetchDPObjects(contractId, type);

    expect(result).to.be.deep.equal([]);
  });
});
