const protocolVersion = require('@dashevo/dpp/lib/version/protocolVersion');

const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');

const createStateRepositoryMock = require('@dashevo/dpp/lib/test/mocks/createStateRepositoryMock');

const { default: loadWasmDpp } = require('../../../dist');
const getBlsAdapterMock = require('../../../lib/test/mocks/getBlsAdapterMock');

describe('StateTransitionFactory', function main() {
  this.timeout(10000);

  let factory;
  let stateTransition;
  let rawStateTransition;
  let stateRepositoryMock;
  let StateTransitionFactory;

  before(async () => {
    ({
      DataContractCreateTransition, StateTransitionFactory,
      UnsupportedProtocolVersionError, SerializedObjectParsingError,
    } = await loadWasmDpp());
  });

  beforeEach(async function beforeEach() {
    const dataContract = getDataContractFixture();

    const rawDataContract = dataContract.toObject();

    stateTransition = new DataContractCreateTransition({
      protocolVersion: protocolVersion.latestVersion,
      dataContract: rawDataContract,
      entropy: dataContract.getEntropy(),
      signature: Buffer.alloc(65),
      signaturePublicKeyId: 0,
    });
    rawStateTransition = stateTransition.toObject();

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);

    const blsAdapter = await getBlsAdapterMock();

    factory = new StateTransitionFactory(
      stateRepositoryMock,
      blsAdapter,
    );
  });

  describe('createFromObject', () => {
    it('should return new State Transition with data from passed object', async () => {
      const result = await factory.createFromObject(rawStateTransition);

      expect(result).to.equal(stateTransition);
    });

    it('should return new State Transition without validation if "skipValidation" option is passed', async () => {
      const result = await factory.createFromObject(rawStateTransition, { skipValidation: true });

      expect(result).to.equal(stateTransition);
    });
  });

  describe('createFromBuffer', () => {
    let serializedStateTransition;

    it('should return new State Transition from serialized contract', async () => {
      const result = await factory.createFromBuffer(serializedStateTransition);

      expect(result).to.equal(stateTransition);
    });
  });
});
