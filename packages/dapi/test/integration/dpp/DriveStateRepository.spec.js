const chai = require('chai');
const sinon = require('sinon');

const chaiAsPromised = require('chai-as-promised');
const dirtyChai = require('dirty-chai');

const { default: loadWasmDpp, DashPlatformProtocol } = require('@dashevo/wasm-dpp');

const generateRandomIdentifierAsync = require('@dashevo/wasm-dpp/lib/test/utils/generateRandomIdentifierAsync');
const getDataContractFixture = require('@dashevo/wasm-dpp/lib/test/fixtures/getDataContractFixture');

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

  before(async () => {
    await loadWasmDpp();
  });

  beforeEach(async function before() {
    dataContractFixture = await getDataContractFixture();

    dpp = new DashPlatformProtocol(null, 1);

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
      const contractId = await generateRandomIdentifierAsync();
      const result = await stateRepository.fetchDataContract(contractId);

      expect(result.toObject()).to.be.deep.equal(dataContractFixture.toObject());

      expect(driveClientMock.fetchDataContract).to.be.calledOnce();
      expect(driveClientMock.fetchDataContract).to.be.calledWithExactly(contractId, false);
    });
  });
});
