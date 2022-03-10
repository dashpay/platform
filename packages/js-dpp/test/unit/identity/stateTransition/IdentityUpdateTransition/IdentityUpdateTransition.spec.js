const IdentityPublicKey = require('../../../../../lib/identity/IdentityPublicKey');

const stateTransitionTypes = require(
  '../../../../../lib/stateTransition/stateTransitionTypes',
);

const protocolVersion = require('../../../../../lib/version/protocolVersion');
const IdentityUpdateTransition = require('../../../../../lib/identity/stateTransition/IdentityUpdateTransition/IdentityUpdateTransition');
const Identifier = require('../../../../../lib/identifier/Identifier');

const getIdentityUpdateTransitionFixture = require('../../../../../lib/test/fixtures/getIdentityUpdateTransitionFixture');

describe('IdentityUpdateTransition', () => {
  let rawStateTransition;
  let stateTransition;

  beforeEach(() => {
    stateTransition = getIdentityUpdateTransitionFixture();
    rawStateTransition = stateTransition.toObject();
  });

  describe('#constructor', () => {
    it('should create an instance with specified data from specified raw transition', () => {

    });
  });

  describe('#getType', () => {
    it('should return IDENTITY_UPDATE type', () => {
      expect(stateTransition.getType()).to.equal(stateTransitionTypes.IDENTITY_UPDATE);
    });
  });

  describe('#setIdentityId', () => {

  });

  describe('#getIdentityId', () => {

  });

  describe('#getRevision', () => {

  });

  describe('#setRevision', () => {

  });

  describe('#getOwnerId', () => {
    it('should return owner id', () => {
      expect(stateTransition.getOwnerId()).to.deep.equal(
        rawStateTransition.identityId,
      );
    });
  });

  describe('#getAddPublicKeys', () => {

  });

  describe('#setAddPublicKeys', () => {

  });

  describe('#getDisablePublicKeys', () => {

  });

  describe('#setDisablePublicKeys', () => {

  });

  describe('#getPublicKeysDisabledAt', () => {

  });

  describe('#setPublicKeysDisabledAt', () => {

  });

  describe('#toObject', () => {
    
  });

  describe('#toJSON ', () => {
    
  });

  describe('#getModifiedDataIds', () => {
    it('should return ids of topped up identity', () => {
      const result = stateTransition.getModifiedDataIds();

      expect(result.length).to.be.equal(1);
      const identityId = result[0];

      expect(identityId).to.be.an.instanceOf(Identifier);
      expect(identityId).to.be.deep.equal(rawStateTransition.identityId);
    });
  });

  describe('#isDataContractStateTransition', () => {
    it('should return false', () => {
      expect(stateTransition.isDataContractStateTransition()).to.be.false();
    });
  });

  describe('#isDocumentStateTransition', () => {
    it('should return false', () => {
      expect(stateTransition.isDocumentStateTransition()).to.be.false();
    });
  });

  describe('#isIdentityStateTransition', () => {
    it('should return true', () => {
      expect(stateTransition.isIdentityStateTransition()).to.be.true();
    });
  });
});
