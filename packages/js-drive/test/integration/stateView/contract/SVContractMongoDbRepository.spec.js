const { mocha: { startMongoDb } } = require('@dashevo/dp-services-ctl');

const DashPlatformProtocol = require('@dashevo/dpp');

const SVContractMongoDbRepository = require('../../../../lib/stateView/contract/SVContractMongoDbRepository');

const getSVContractFixture = require('../../../../lib/test/fixtures/getSVContractFixture');

const MongoDBTransaction = require('../../../../lib/mongoDb/MongoDBTransaction');

describe('SVContractMongoDbRepository', () => {
  let svContractRepository;
  let svContract;
  let mongoDatabase;
  let mongoClient;
  let stateViewTransaction;

  startMongoDb().then((mongoDb) => {
    mongoDatabase = mongoDb.getDb();
    mongoClient = mongoDb.getClient();
  });

  beforeEach(() => {
    const dpp = new DashPlatformProtocol();

    svContract = getSVContractFixture();

    svContractRepository = new SVContractMongoDbRepository(mongoDatabase, dpp);

    stateViewTransaction = new MongoDBTransaction(mongoClient);
  });

  it('should store SV Contract entity', async () => {
    await svContractRepository.store(svContract);

    const result = await svContractRepository.find(svContract.getContractId());

    expect(result.toJSON()).to.deep.equal(svContract.toJSON());
  });

  it('should return null if SV Contract is not found', async () => {
    const result = await svContractRepository.find('unknown');

    expect(result).to.be.null();
  });

  it('should find all contracts by stHash', async () => {
    await svContractRepository.store(svContract);

    const svContracts = await svContractRepository.findAllByReferenceSTHash(
      svContract.getReference().getSTHash(),
    );

    expect(svContracts.length).to.equal(1);
    expect(svContracts).to.deep.equal([svContract]);
  });

  it('should find all contracts by stHash in transaction', async () => {
    await svContractRepository.createCollection();

    stateViewTransaction.start();

    await svContractRepository.store(svContract, stateViewTransaction);

    const svContracts = await svContractRepository.findAllByReferenceSTHash(
      svContract.getReference().getSTHash(),
      stateViewTransaction,
    );

    await stateViewTransaction.commit();

    expect(svContracts.length).to.equal(1);
    expect(svContracts).to.deep.equal([svContract]);
  });

  it('should return null if contract was marked as deleted', async () => {
    svContract.markAsDeleted();

    await svContractRepository.store(svContract);

    const contract = await svContractRepository.find(svContract.getContractId());

    expect(contract).to.be.null();
  });

  it('should use base58-encoded Contract ID as MongoDB document ID', async () => {
    await svContractRepository.store(svContract);

    const result = await mongoDatabase.collection('contracts').findOne({
      _id: svContract.getContractId(),
    });

    expect(result).to.be.not.null();
  });

  it('should successfully delete a contract', async () => {
    await svContractRepository.store(svContract);

    const svContracts = await svContractRepository.findAllByReferenceSTHash(
      svContract.getReference().getSTHash(),
    );

    await svContractRepository.delete(svContract);

    const emptySVContracts = await svContractRepository.findAllByReferenceSTHash(
      svContract.getReference().getSTHash(),
    );

    expect(svContracts.length).to.equal(1);
    expect(emptySVContracts.length).to.equal(0);
  });

  it('should successfully delete a contract in transaction', async () => {
    await svContractRepository.createCollection();

    stateViewTransaction.start();

    await svContractRepository.store(svContract, stateViewTransaction);

    const svContracts = await svContractRepository.findAllByReferenceSTHash(
      svContract.getReference().getSTHash(),
      stateViewTransaction,
    );

    await svContractRepository.delete(svContract, stateViewTransaction);

    const emptySVContracts = await svContractRepository.findAllByReferenceSTHash(
      svContract.getReference().getSTHash(),
      stateViewTransaction,
    );

    await stateViewTransaction.commit();

    expect(svContracts.length).to.equal(1);
    expect(emptySVContracts.length).to.equal(0);
  });

  it('should create collection for SVContract', async () => {
    const collectionsBefore = await mongoDatabase.collections();
    await svContractRepository.createCollection();
    const collectionsAfter = await mongoDatabase.collections();

    expect(collectionsBefore).to.have.lengthOf(0);
    expect(collectionsAfter).to.have.lengthOf(1);
    expect(collectionsAfter[0].collectionName).to.equal(svContractRepository.getCollectionName());
  });

  it('should find stored contract by id in transaction', async () => {
    await svContractRepository.createCollection();

    stateViewTransaction.start();

    await svContractRepository.store(svContract, stateViewTransaction);

    const storedSVContract = await svContractRepository.find(
      svContract.getContractId(),
      stateViewTransaction,
    );

    await stateViewTransaction.commit();

    expect(storedSVContract).to.deep.equal(svContract);
  });

  it('should find stored contract by id', async () => {
    await svContractRepository.store(svContract);

    const storedSVContract = await svContractRepository.find(svContract.getContractId());

    expect(storedSVContract).to.deep.equal(svContract);
  });
});
