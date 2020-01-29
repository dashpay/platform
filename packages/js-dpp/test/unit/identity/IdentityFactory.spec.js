const rewiremock = require('rewiremock/node');

const Identity = require('../../../lib/identity/Identity');

const getIdentityFixture = require('../../../lib/test/fixtures/getIdentityFixture');

const ValidationResult = require('../../../lib/validation/ValidationResult');
const ConsensusError = require('../../../lib/errors/ConsensusError');

const InvalidIdentityError = require(
  '../../../lib/identity/errors/InvalidIdentityError',
);

describe('IdentityFactory', () => {
  let factory;
  let validateIdentityMock;
  let decodeMock;
  let IdentityFactory;
  let identity;

  beforeEach(function beforeEach() {
    validateIdentityMock = this.sinonSandbox.stub();
    decodeMock = this.sinonSandbox.stub();

    IdentityFactory = rewiremock.proxy(
      '../../../lib/identity/IdentityFactory',
      {
        '../../../lib/util/serializer': {
          decode: decodeMock,
        },
        '../../../lib/identity/Identity': Identity,
      },
    );

    factory = new IdentityFactory(validateIdentityMock);

    identity = getIdentityFixture();
  });

  describe('#constructor', () => {
    it('should set validator', () => {
      expect(factory.validateIdentity).to.equal(validateIdentityMock);
    });
  });

  describe('#create', () => {
    it('should create Identity with specified id, type and public keys', () => {
      const result = factory.create(
        identity.getId(),
        identity.getType(),
        identity.getPublicKeys(),
      );

      expect(result).to.be.an.instanceOf(Identity);
      expect(result).to.deep.equal(identity);
    });
  });

  describe('#createFromObject', () => {
    it('should skip validation if options is set', () => {
      factory.createFromObject({}, { skipValidation: true });

      expect(validateIdentityMock).to.have.not.been.called();
    });

    it('should throw an error if validation have failed', () => {
      const errors = [new ConsensusError('error')];

      validateIdentityMock.returns(new ValidationResult(errors));

      try {
        factory.createFromObject(identity.toJSON());

        expect.fail('error was not thrown');
      } catch (e) {
        expect(e).to.be.an.instanceOf(InvalidIdentityError);
        expect(e.getErrors()).to.have.deep.members(errors);
        expect(e.getRawIdentity()).to.deep.equal(identity.toJSON());
      }
    });

    it('should create an identity if validation passed', () => {
      validateIdentityMock.returns(new ValidationResult());

      const result = factory.createFromObject(identity.toJSON());

      expect(result).to.be.an.instanceOf(Identity);
      expect(result).to.deep.equal(identity);
    });
  });

  describe('#createFromSerialized', () => {
    beforeEach(function beforeEach() {
      this.sinonSandbox.stub(factory, 'createFromObject');
    });

    it('should return new Identity from serialized one', () => {
      const serializedIdentity = identity.serialize();

      decodeMock.returns(identity.toJSON());

      factory.createFromObject.returns(identity);

      const result = factory.createFromSerialized(serializedIdentity);

      expect(result).to.equal(identity);

      expect(factory.createFromObject).to.have.been.calledOnceWith(identity.toJSON());

      expect(decodeMock).to.have.been.calledOnceWith(serializedIdentity);
    });
  });
});
