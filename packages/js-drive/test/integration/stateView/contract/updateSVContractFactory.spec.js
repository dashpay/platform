const { mocha: { startMongoDb } } = require('@dashevo/dp-services-ctl');

const DashPlatformProtocol = require('@dashevo/dpp');

const SVContract = require('../../../../lib/stateView/contract/SVContract');

const SVContractMongoDbRepository = require('../../../../lib/stateView/contract/SVContractMongoDbRepository');

const updateSVContractFactory = require('../../../../lib/stateView/contract/updateSVContractFactory');

const getSVContractFixture = require('../../../../lib/test/fixtures/getSVContractFixture');
const getReferenceFixture = require('../../../../lib/test/fixtures/getReferenceFixture');

describe('updateSVContractFactory', () => {
  let svContractRepository;
  let updateSVContract;
  let mongoDatabase;
  let svContract;
  let dpp;
  let contractId;
  let contract;

  startMongoDb().then((mongoDb) => {
    mongoDatabase = mongoDb.getDb();
  });

  beforeEach(() => {
    dpp = new DashPlatformProtocol();

    svContract = getSVContractFixture();
    contract = svContract.getDataContract();

    contractId = svContract.getId();

    svContractRepository = new SVContractMongoDbRepository(mongoDatabase, dpp);
    updateSVContract = updateSVContractFactory(svContractRepository);
  });

  it('should store SVContract', async () => {
    await updateSVContract(
      svContract.getDataContract(),
      svContract.getReference(),
    );

    const fetchedSVContract = await svContractRepository.find(svContract.getId());

    expect(fetchedSVContract).to.deep.equal(svContract);
  });

  it('should maintain SVContract previous revisions and add new one', async () => {
    // Create and store the second contract version
    const secondDPOContract = dpp.dataContract.createFromObject(contract.toJSON());
    secondDPOContract.setVersion(2);

    const secondSVContract = new SVContract(
      contract,
      getReferenceFixture(2),
      false,
      [svContract.getCurrentRevision()],
    );

    await svContractRepository.store(secondSVContract);

    // Update to the third contract version
    const thirdContract = dpp.dataContract.createFromObject(contract.toJSON());
    thirdContract.setVersion(3);

    await updateSVContract(
      thirdContract,
      getReferenceFixture(3),
    );

    const thirdSVContract = await svContractRepository.find(contractId);

    expect(thirdSVContract).to.be.an.instanceOf(SVContract);
    expect(thirdSVContract.getDataContract()).to.deep.equal(thirdContract);
    expect(thirdSVContract.getPreviousRevisions()).to.deep.equal([
      svContract.getCurrentRevision(),
      secondSVContract.getCurrentRevision(),
    ]);
  });
});
