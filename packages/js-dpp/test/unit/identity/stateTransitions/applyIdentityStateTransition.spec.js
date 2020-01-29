const Identity = require('../../../../lib/identity/Identity');

const applyIdentityStateTransition = require('../../../../lib/identity/stateTransitions/applyIdentityStateTransition');

const getIdentityCreateSTFixture = require('../../../../lib/test/fixtures/getIdentityCreateSTFixture');

const IdentityAlreadyExistsError = require('../../../../lib/errors/IdentityAlreadyExistsError');
const WrongStateTransitionTypeError = require('../../../../lib/identity/errors/WrongStateTransitionTypeError');

describe('applyIdentityStateTransition', () => {
  describe('Identity Create', () => {
    let createStateTransition;

    beforeEach(() => {
      createStateTransition = getIdentityCreateSTFixture();
    });

    it('should throw an error if identity is already present', () => {
      const identity = new Identity();

      try {
        applyIdentityStateTransition(createStateTransition, identity);

        expect.fail('error was not thrown');
      } catch (e) {
        expect(e).to.be.an.instanceOf(IdentityAlreadyExistsError);
        expect(e.getStateTransition()).to.equal(createStateTransition);
      }
    });

    it('should set proper data from state transition', () => {
      const identity = applyIdentityStateTransition(createStateTransition, null);

      expect(identity.getId()).to.equal(createStateTransition.getIdentityId());
      expect(identity.getType()).to.equal(createStateTransition.getIdentityType());
      expect(identity.getPublicKeys()).to.deep.equal(createStateTransition.getPublicKeys());
    });
  });

  it('should throw an error if state transition is of wrong type', function it() {
    const createStateTransition = getIdentityCreateSTFixture();
    this.sinonSandbox.stub(createStateTransition, 'getType').returns(42);

    try {
      applyIdentityStateTransition(createStateTransition, null);

      expect.fail('error was not thrown');
    } catch (e) {
      expect(e).to.be.an.instanceOf(WrongStateTransitionTypeError);
      expect(e.getStateTransition()).to.equal(createStateTransition);
    }
  });
});
