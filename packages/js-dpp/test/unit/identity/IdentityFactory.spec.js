const rewiremock = require('rewiremock/node');
const crypto = require('crypto');

const { PublicKey } = require('@dashevo/dashcore-lib');

const Identifier = require('../../../lib/Identifier');

const hash = require('../../../lib/util/hash');

const Identity = require('../../../lib/identity/Identity');
const IdentityCreateTransition = require('../../../lib/identity/stateTransitions/identityCreateTransition/IdentityCreateTransition');
const IdentityTopUpTransition = require('../../../lib/identity/stateTransitions/identityTopUpTransition/IdentityTopUpTransition');

const getIdentityFixture = require('../../../lib/test/fixtures/getIdentityFixture');

const ValidationResult = require('../../../lib/validation/ValidationResult');
const ConsensusError = require('../../../lib/errors/ConsensusError');
const SerializedObjectParsingError = require('../../../lib/errors/SerializedObjectParsingError');

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
        '../../../lib/identity/stateTransitions/identityCreateTransition/IdentityCreateTransition': IdentityCreateTransition,
        '../../../lib/identity/stateTransitions/identityTopUpTransition/IdentityTopUpTransition': IdentityTopUpTransition,
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
    it('should create Identity from transaction out point and public keys', () => {
      const lockedOutPoint = crypto.randomBytes(64);

      identity.id = Identifier.from(
        hash(lockedOutPoint),
      );

      identity.setBalance(0);

      const publicKeys = identity.getPublicKeys().map((identityPublicKey) => {
        const publicKeyData = Buffer.from(identityPublicKey.getData(), 'base64');

        return new PublicKey(publicKeyData);
      });

      const result = factory.create(
        lockedOutPoint,
        publicKeys,
      );

      expect(result).to.be.an.instanceOf(Identity);
      expect(result.toObject()).to.deep.equal(identity.toObject());
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
        factory.createFromObject(identity.toObject());

        expect.fail('error was not thrown');
      } catch (e) {
        expect(e).to.be.an.instanceOf(InvalidIdentityError);
        expect(e.getErrors()).to.have.deep.members(errors);
        expect(e.getRawIdentity()).to.deep.equal(identity.toObject());
      }
    });

    it('should create an identity if validation passed', () => {
      validateIdentityMock.returns(new ValidationResult());

      const result = factory.createFromObject(identity.toObject());

      expect(result).to.be.an.instanceOf(Identity);
      expect(result).to.deep.equal(identity);
    });
  });

  describe('#createFromBuffer', () => {
    beforeEach(function beforeEach() {
      this.sinonSandbox.stub(factory, 'createFromObject');
    });

    it('should return new Identity from serialized one', () => {
      const serializedIdentity = identity.toBuffer();

      decodeMock.returns(identity.toObject());

      factory.createFromObject.returns(identity);

      const result = factory.createFromBuffer(serializedIdentity);

      expect(result).to.equal(identity);

      expect(factory.createFromObject).to.have.been.calledOnceWith(identity.toObject());

      expect(decodeMock).to.have.been.calledOnceWith(serializedIdentity);
    });

    it('should throw consensus error if `decode` fails', () => {
      const parsingError = new Error('Something failed during parsing');

      const serializedIdentity = identity.toBuffer();

      decodeMock.throws(parsingError);

      try {
        factory.createFromBuffer(serializedIdentity);
        expect.fail('Error was not thrown');
      } catch (e) {
        expect(e).to.be.an.instanceOf(InvalidIdentityError);

        const [innerError] = e.getErrors();
        expect(innerError).to.be.an.instanceOf(SerializedObjectParsingError);
        expect(innerError.getPayload()).to.deep.equal(serializedIdentity);
        expect(innerError.getParsingError()).to.deep.equal(parsingError);
      }
    });
  });

  describe('#createIdentityCreateTransition', () => {
    it('should create IdentityCreateTransition from Identity model', () => {
      const lockedOutPoint = crypto.randomBytes(64);

      identity.setLockedOutPoint(lockedOutPoint);

      const stateTransition = factory.createIdentityCreateTransition(identity);

      expect(stateTransition).to.be.instanceOf(IdentityCreateTransition);
      expect(stateTransition.getPublicKeys()).to.equal(identity.getPublicKeys());
      expect(stateTransition.getLockedOutPoint()).to.deep.equal(lockedOutPoint);
    });
  });

  describe('#createIdentityTopUpTransition', () => {
    it('should create IdentityTopUpTransition from identity id and outpoint', () => {
      const lockedOutPoint = crypto.randomBytes(64);

      identity.setLockedOutPoint(lockedOutPoint);

      const stateTransition = factory
        .createIdentityTopUpTransition(identity.getId(), lockedOutPoint);

      expect(stateTransition).to.be.instanceOf(IdentityTopUpTransition);
      expect(stateTransition.getIdentityId()).to.deep.equal(identity.getId());
      expect(stateTransition.getLockedOutPoint()).to.deep.equal(lockedOutPoint);
    });
  });
});
