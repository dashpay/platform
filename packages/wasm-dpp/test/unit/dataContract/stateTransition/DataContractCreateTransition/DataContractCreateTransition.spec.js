const varint = require('varint');
const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const stateTransitionTypes = require('@dashevo/dpp/lib/stateTransition/stateTransitionTypes');

const protocolVersion = require('@dashevo/dpp/lib/version/protocolVersion');

const { default: loadWasmDpp } = require('../../../../../dist');

describe('DataContractCreateTransition', () => {
  let stateTransition;
  let dataContract;
  let DataContractCreateTransition;
  let Identifier;

  before(async () => {
    ({
      DataContractCreateTransition, Identifier,
    } = await loadWasmDpp());
  });

  beforeEach(() => {
    dataContract = getDataContractFixture();

    stateTransition = new DataContractCreateTransition({
      protocolVersion: protocolVersion.latestVersion,
      dataContract: dataContract.toObject(),
      entropy: dataContract.getEntropy(),
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

      expect(result).to.equal(stateTransitionTypes.DATA_CONTRACT_CREATE);
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
        type: stateTransitionTypes.DATA_CONTRACT_CREATE,
        dataContract: dataContract.toJSON(),
        entropy: dataContract.getEntropy().toString('base64'),
      });
    });
  });

  describe('#toBuffer', () => {
    it('should return serialized State Transition', () => {
      const protocolVersionBytes = Buffer.from(varint.encode(stateTransition.getProtocolVersion()));

      const result = stateTransition.toBuffer();
      expect(result.compare(protocolVersionBytes, 0, 4, 0, 4)).equals(0);
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
