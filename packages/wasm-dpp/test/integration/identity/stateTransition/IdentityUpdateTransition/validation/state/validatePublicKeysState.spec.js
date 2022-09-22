const validatePublicKeys = require(
  '../../../../../../../lib/identity/stateTransition/IdentityUpdateTransition/validation/state/validatePublicKeysState',
);
const { expectValidationError } = require('../../../../../../../lib/test/expect/expectError');
const DuplicatedIdentityPublicKeyIdError = require('../../../../../../../lib/errors/consensus/state/identity/DuplicatedIdentityPublicKeyIdError');
const DuplicatedIdentityPublicKeyError = require('../../../../../../../lib/errors/consensus/state/identity/DuplicatedIdentityPublicKeyError');
const identitySchema = require('../../../../../../../schema/identity/identity.json');
const MaxIdentityPublicKeyLimitReachedError = require('../../../../../../../lib/errors/consensus/state/identity/MaxIdentityPublicKeyLimitReachedError');
const getIdentityFixture = require('../../../../../../../lib/test/fixtures/getIdentityFixture');

describe('validatePublicKeysState', () => {
  let rawPublicKeys;

  beforeEach(() => {
    ({ publicKeys: rawPublicKeys } = getIdentityFixture().toObject());
  });

  it('should return invalid result if there are duplicate key ids', () => {
    rawPublicKeys[1].id = rawPublicKeys[0].id;

    const result = validatePublicKeys(rawPublicKeys);

    expectValidationError(result, DuplicatedIdentityPublicKeyIdError);

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(4022);
    expect(error.getDuplicatedIds()).to.deep.equal([rawPublicKeys[1].id]);
  });

  it('should return invalid result if there are duplicate keys', () => {
    rawPublicKeys[1].data = rawPublicKeys[0].data;

    const result = validatePublicKeys(rawPublicKeys);

    expectValidationError(result, DuplicatedIdentityPublicKeyError);

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(4021);
    expect(error.getDuplicatedPublicKeysIds()).to.deep.equal([rawPublicKeys[1].id]);
  });

  it('should pass valid public keys', () => {
    const result = validatePublicKeys(rawPublicKeys);

    expect(result.isValid()).to.be.true();
  });

  it('should return invalid result if number of public keys is bigger than 32', () => {
    const { maxItems } = identitySchema.properties.publicKeys;
    const numToAdd = maxItems - rawPublicKeys.length;

    for (let i = 0; i <= numToAdd; ++i) {
      rawPublicKeys.push(rawPublicKeys[0]);
    }

    const result = validatePublicKeys(rawPublicKeys);

    expectValidationError(result, MaxIdentityPublicKeyLimitReachedError);

    const [error] = result.getErrors();
    expect(error.getCode()).to.equal(4020);
    expect(error.geMaxItems()).to.equal(maxItems);
  });
});
