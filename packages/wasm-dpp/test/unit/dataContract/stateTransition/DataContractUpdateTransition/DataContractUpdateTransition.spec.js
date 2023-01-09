const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const stateTransitionTypes = require('@dashevo/dpp/lib/stateTransition/stateTransitionTypes');
const protocolVersion = require('@dashevo/dpp/lib/version/protocolVersion');
const JsDataContractUpdateTransition = require('@dashevo/dpp/lib/dataContract/stateTransition/DataContractUpdateTransition/DataContractUpdateTransition');

const { default: loadWasmDpp } = require('../../../../../dist');

describe('DataContractUpdateTransition', () => {
  let stateTransition;
  let dataContract;
  let DataContractUpdateTransition;
  let Identifier;

  before(async () => {
    ({
      DataContractUpdateTransition, Identifier,
    } = await loadWasmDpp());
  });

  beforeEach(() => {
    dataContract = getDataContractFixture();

    stateTransition = new DataContractUpdateTransition({
      protocolVersion: protocolVersion.latestVersion,
      dataContract: dataContract.toObject(),
    });
  });

  describe('#getProtocolVersion', () => {
    it('should return the current protocol version', () => {
      const result = stateTransition.getProtocolVersion();

      expect(result).to.equal(protocolVersion.latestVersion);
    });
  });

  describe('#getType', () => {
    it('should return State Transition type', () => {
      const result = stateTransition.getType();

      expect(result).to.equal(stateTransitionTypes.DATA_CONTRACT_UPDATE);
    });
  });

  describe('#getDataContract', () => {
    it('should return Data Contract', () => {
      const result = stateTransition.getDataContract();

      expect(result.toObject()).to.deep.equal(dataContract.toObject());
    });
  });

  describe('#toJSON', () => {
    it('should return State Transition as plain JS object', () => {
      expect(stateTransition.toJSON(true)).to.deep.equal({
        protocolVersion: protocolVersion.latestVersion,
        type: stateTransitionTypes.DATA_CONTRACT_UPDATE,
        dataContract: dataContract.toJSON(),
      });
    });
  });

  describe('#toBuffer', () => {
    it('should return serialized State Transition that starts with protocol version', () => {
      const protocolVersionUInt32 = Buffer.alloc(4);
      protocolVersionUInt32.writeUInt32LE(stateTransition.getProtocolVersion(), 0);

      const result = stateTransition.toBuffer();
      expect(result.compare(protocolVersionUInt32, 0, 4, 0, 4)).equals(0);
    });
  });

  describe.skip('#hash', () => {
    it('should return State Transition hash as hex', () => {
      const jsStateTransition = new JsDataContractUpdateTransition(stateTransition.toJSON());

      const result = stateTransition.hash();
      const resultJs = jsStateTransition.hash();

      expect(result).to.equal(resultJs);
    });
  });

  describe('#getOwnerId', () => {
    it('should return owner id', async () => {
      const result = stateTransition.getOwnerId();
      const reference = stateTransition.getDataContract().getOwnerId();

      expect(result.toBuffer()).to.deep.equal(reference.toBuffer());
    });
  });

  describe('#getModifiedDataIds', () => {
    it('should return ids of affected data contracts', () => {
      const result = stateTransition.getModifiedDataIds();

      expect(result.length).to.be.equal(1);
      const contractId = result[0];

      expect(contractId).to.be.an.instanceOf(Identifier);
      expect(contractId.toBuffer()).to.be.deep.equal(dataContract.getId().toBuffer());
    });
  });

  describe('#isDataContractStateTransition', () => {
    it('should return true', () => {
      expect(stateTransition.isDataContractStateTransition()).to.be.true();
    });
  });

  describe('#isDocumentStateTransition', () => {
    it('should return false', () => {
      expect(stateTransition.isDocumentStateTransition()).to.be.false();
    });
  });

  describe('#isIdentityStateTransition', () => {
    it('should return false', () => {
      expect(stateTransition.isIdentityStateTransition()).to.be.false();
    });
  });
});
