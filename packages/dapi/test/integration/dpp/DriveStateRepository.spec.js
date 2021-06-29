const chai = require('chai');
const sinon = require('sinon');

const chaiAsPromised = require('chai-as-promised');
const dirtyChai = require('dirty-chai');

const DashPlatformProtocol = require('@dashevo/dpp');

const generateRandomIdentifier = require('@dashevo/dpp/lib/test/utils/generateRandomIdentifier');
const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');

const {
  v0: {
    GetDataContractResponse,
  },
} = require('@dashevo/dapi-grpc');

const DriveStateRepository = require('../../../lib/dpp/DriveStateRepository');


chai.use(chaiAsPromised);
chai.use(dirtyChai);

const { expect } = chai;

describe('DriveStateRepository', () => {
  let dpp;
  let driveClientMock;
  let stateRepository;
  let dataContractFixture;
  let proto;

  beforeEach(async function before() {
    dataContractFixture = getDataContractFixture();

    dpp = new DashPlatformProtocol();
    await dpp.initialize();
    sinon.spy(dpp.dataContract, 'createFromBuffer');

    proto = new GetDataContractResponse();
    proto.setDataContract(dataContractFixture.toBuffer());

    driveClientMock = sinon.stub();
    driveClientMock.fetchDataContract = this.sinon.stub().resolves(
      proto.serializeBinary(),
    );

    stateRepository = new DriveStateRepository(driveClientMock, dpp);
  });

  describe('#fetchDataContract', () => {
    it('should fetch and parse data contract', async () => {
      const contractId = generateRandomIdentifier();
      const result = await stateRepository.fetchDataContract(contractId);

      expect(result.toObject()).to.be.deep.equal(dataContractFixture.toObject());

      expect(dpp.dataContract.createFromBuffer).to.be.calledOnce();
      expect(dpp.dataContract.createFromBuffer.getCall(0).args).to.have.deep.members([
        proto.getDataContract_asU8(),
      ]);

      expect(driveClientMock.fetchDataContract).to.be.calledOnce();
      expect(driveClientMock.fetchDataContract).to.be.calledWithExactly(contractId, false);
    });
  });
});
