const DashPlatformProtocol = require('../../../lib/DashPlatformProtocol');

const Identity = require('../../../lib/identity/Identity');

const ValidationResult = require('../../../lib/validation/ValidationResult');

const getIdentityFixture = require('../../../lib/test/fixtures/getIdentityFixture');
const getIdentityCreateSTFixture = require('../../../lib/test/fixtures/getIdentityCreateSTFixture');

describe('IdentityFacade', () => {
  let dpp;
  let identity;

  beforeEach(() => {
    dpp = new DashPlatformProtocol();

    identity = getIdentityFixture();
  });

  describe('#create', () => {
    it('should create Identity', () => {
      const result = dpp.identity.create(
        identity.getId(),
        identity.getType(),
        identity.getPublicKeys(),
      );

      expect(result).to.be.an.instanceOf(Identity);

      expect(result).to.deep.equal(identity);
    });
  });

  describe('#createFromObject', () => {
    it('should create Identity from plain object', () => {
      const result = dpp.identity.createFromObject(identity.toJSON());

      expect(result).to.be.an.instanceOf(Identity);

      expect(result).to.deep.equal(identity);
    });
  });

  describe('#createFromSerialized', () => {
    it('should create Identity from string', () => {
      const result = dpp.identity.createFromSerialized(identity.serialize());

      expect(result).to.be.an.instanceOf(Identity);

      expect(result).to.deep.equal(identity);
    });
  });

  describe('#validate', () => {
    it('should validate Identity', async () => {
      const result = await dpp.identity.validate(identity);

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();
    });
  });

  describe('#applyStateTransition', () => {
    it('should apply identity create transition', () => {
      const createStateTransition = getIdentityCreateSTFixture();
      const result = dpp.identity.applyStateTransition(createStateTransition, null);

      expect(result).to.be.an.instanceOf(Identity);
    });
  });
});
