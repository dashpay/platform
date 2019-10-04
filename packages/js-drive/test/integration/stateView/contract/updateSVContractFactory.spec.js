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
  let userId;
  let contract;

  startMongoDb().then((mongoDb) => {
    mongoDatabase = mongoDb.getDb();
  });

  beforeEach(() => {
    dpp = new DashPlatformProtocol();

    svContract = getSVContractFixture();
    contract = svContract.getContract();

    contractId = svContract.getContractId();
    userId = svContract.getUserId();

    svContractRepository = new SVContractMongoDbRepository(mongoDatabase, dpp);
    updateSVContract = updateSVContractFactory(svContractRepository);
  });

  it('should store SVContract', async () => {
    await updateSVContract(
      svContract.getContractId(),
      svContract.getUserId(),
      svContract.getReference(),
      svContract.getContract(),
    );

    const fetchedSVContract = await svContractRepository.find(svContract.getContractId());

    expect(fetchedSVContract).to.deep.equal(svContract);
  });

  it('should maintain SVContract previous revisions and add new one', async () => {
    // Create and store the second contract version
    const secondDPOContract = dpp.contract.createFromObject(contract.toJSON());
    secondDPOContract.setVersion(2);

    const secondSVContract = new SVContract(
      contractId,
      userId,
      contract,
      getReferenceFixture(2),
      false,
      [svContract.getCurrentRevision()],
    );

    await svContractRepository.store(secondSVContract);

    // Update to the third contract version
    const thirdContract = dpp.contract.createFromObject(contract.toJSON());
    thirdContract.setVersion(3);

    await updateSVContract(
      contractId,
      userId,
      getReferenceFixture(3),
      thirdContract,
    );

    const thirdSVContract = await svContractRepository.find(contractId);

    expect(thirdSVContract).to.be.an.instanceOf(SVContract);
    expect(thirdSVContract.getContract()).to.deep.equal(thirdContract);
    expect(thirdSVContract.getPreviousRevisions()).to.deep.equal([
      svContract.getCurrentRevision(),
      secondSVContract.getCurrentRevision(),
    ]);
  });
});
