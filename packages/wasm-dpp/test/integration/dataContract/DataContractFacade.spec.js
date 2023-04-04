const getDataContractJSFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const crypto = require('crypto');
const getBlsAdapterMock = require('../../../lib/test/mocks/getBlsAdapterMock');
const createStateRepositoryMock = require('../../../lib/test/mocks/createStateRepositoryMock');
const getPrivateAndPublicKey = require('../../../lib/test/fixtures/getPrivateAndPublicKeyForSigningFixture');

const { default: loadWasmDpp } = require('../../..');
let {
  DashPlatformProtocol, DataContract, ValidationResult, DataContractValidator,
  DataContractFactory, DataContractCreateTransition, DataContractUpdateTransition,
} = require('../../..');

describe('DataContractFacade', () => {
  let dpp;
  let dataContractJs;
  let dataContractFactory;
  let blsAdapter;
  let rawDataContract;
  let dataContractWasm;
  let stateTransitionMock;

  before(async () => {
    ({
      DashPlatformProtocol, DataContract, ValidationResult,
      DataContractValidator, DataContractFactory, DataContractCreateTransition,
      DataContractUpdateTransition,
    } = await loadWasmDpp());
  });

  beforeEach(async function beforeEach() {
    blsAdapter = await getBlsAdapterMock();
    stateTransitionMock = createStateRepositoryMock(this.sinonSandbox);
    dpp = new DashPlatformProtocol(
      blsAdapter, stateTransitionMock, { generate: () => crypto.randomBytes(32) }, 1,
    );

    dataContractJs = await getDataContractJSFixture();
    rawDataContract = dataContractJs.toObject();

    const dataContractValidator = new DataContractValidator();
    dataContractFactory = new DataContractFactory(
      1,
      dataContractValidator,
    );

    dataContractWasm = await dataContractFactory.createFromObject(rawDataContract);
  });

  describe('create', () => {
    it('should create DataContract', () => {
      const result = dpp.dataContract.create(
        dataContractJs.getOwnerId(),
        dataContractJs.getDocuments(),
      );

      expect(result).to.be.an.instanceOf(DataContract);

      expect(result.getOwnerId().toBuffer()).to.deep.equal(dataContractJs.getOwnerId().toBuffer());
      expect(result.getDocuments()).to.deep.equal(dataContractJs.getDocuments());
    });
  });

  describe('createFromObject', () => {
    it('should create DataContract from plain object', async () => {
      const result = await dpp.dataContract.createFromObject(rawDataContract);

      expect(result).to.be.an.instanceOf(DataContract);

      expect(result.toObject()).to.deep.equal(dataContractJs.toObject());
    });
  });

  describe('createFromBuffer', () => {
    it('should create DataContract from string', async () => {
      const contract = dpp.dataContract.create(
        dataContractJs.getOwnerId(),
        dataContractJs.getDocuments(),
      );

      const result = await dpp.dataContract.createFromBuffer(contract.toBuffer());

      expect(result).to.be.an.instanceOf(DataContract);
      expect(result.toObject()).to.deep.equal(contract.toObject());
    });
  });

  describe('createDataContractCreateTransition', () => {
    it('should create DataContractCreateTransition from DataContract', async () => {
      const stateTransition = await dataContractFactory
        .createDataContractCreateTransition(dataContractWasm);

      const result = dpp.dataContract.createDataContractCreateTransition(dataContractWasm);

      expect(result).to.be.an.instanceOf(DataContractCreateTransition);

      expect(result.toObject()).to.deep.equal(stateTransition.toObject());
    });
  });

  describe('createDataContractUpdateTransition', () => {
    it('should create DataContractUpdateTransition from buffer', async () => {
      const dataContractBuffer = Buffer.from('01a56324696458205a3c9565f215156cfb330ef3a4a4400a1dad7415f4f1480e072d3419bb29c12c6724736368656d61783468747470733a2f2f736368656d612e646173682e6f72672f6470702d302d342d302f6d6574612f646174612d636f6e7472616374676f776e657249645820d5199a5b19334554d5dd483af53877251584ab9a66ca777bd1b2e8d2ec4808346776657273696f6e0169646f63756d656e7473a36c6e696365446f63756d656e74a46474797065666f626a656374687265717569726564816a246372656174656441746a70726f70657274696573a1646e616d65a1647479706566737472696e67746164646974696f6e616c50726f70657274696573f46e7769746842797465417272617973a56474797065666f626a65637467696e646963657381a2646e616d6566696e646578316a70726f7065727469657381a16e6279746541727261794669656c6463617363687265717569726564816e6279746541727261794669656c646a70726f70657274696573a26e6279746541727261794669656c64a36474797065656172726179686d61784974656d731069627974654172726179f56f6964656e7469666965724669656c64a56474797065656172726179686d61784974656d731820686d696e4974656d73182069627974654172726179f570636f6e74656e744d656469615479706578216170706c69636174696f6e2f782e646173682e6470702e6964656e746966696572746164646974696f6e616c50726f70657274696573f46f696e6465786564446f63756d656e74a56474797065666f626a65637467696e646963657386a3646e616d6566696e646578316a70726f7065727469657382a168246f776e6572496463617363a16966697273744e616d656361736366756e69717565f5a3646e616d6566696e646578326a70726f7065727469657382a168246f776e6572496463617363a1686c6173744e616d656361736366756e69717565f5a2646e616d6566696e646578336a70726f7065727469657381a1686c6173744e616d6563617363a2646e616d6566696e646578346a70726f7065727469657382a16a2463726561746564417463617363a16a2475706461746564417463617363a2646e616d6566696e646578356a70726f7065727469657381a16a2475706461746564417463617363a2646e616d6566696e646578366a70726f7065727469657381a16a2463726561746564417463617363687265717569726564846966697273744e616d656a246372656174656441746a24757064617465644174686c6173744e616d656a70726f70657274696573a2686c6173744e616d65a2647479706566737472696e67696d61784c656e677468183f6966697273744e616d65a2647479706566737472696e67696d61784c656e677468183f746164646974696f6e616c50726f70657274696573f4', 'hex');

      const dataContract = await dpp.dataContract.createFromBuffer(dataContractBuffer);

      const updatedDataContract = await dpp.dataContract.createFromObject(
        dataContract.toObject(),
      );

      updatedDataContract.incrementVersion();

      const dataContractUpdateTransition = dpp.dataContract
        .createDataContractUpdateTransition(updatedDataContract);

      const { identityPublicKey, privateKey } = await getPrivateAndPublicKey();

      dataContractUpdateTransition.sign(
        identityPublicKey,
        privateKey,
        await getBlsAdapterMock(),
      );

      const buf = dataContractUpdateTransition.toBuffer();
      stateTransitionMock.fetchDataContract.resolves(dataContract);

      const st = await dpp.stateTransition.createFromBuffer(buf);

      expect(st).to.be.an.instanceOf(DataContractUpdateTransition);
    });
  });

  describe('validate', () => {
    it('should validate DataContract', async () => {
      const result = await dpp.dataContract.validate(rawDataContract);

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.getErrors().length).to.be.equal(0);
    });
  });
});
