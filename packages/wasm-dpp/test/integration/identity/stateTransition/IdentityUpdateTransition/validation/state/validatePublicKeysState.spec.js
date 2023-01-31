const identitySchema = require('@dashevo/dpp/schema/identity/identity.json');
const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');
const { expectValidationError } = require('../../../../../../../lib/test/expect/expectError');

const { default: loadWasmDpp } = require('../../../../../../../dist');

describe('validatePublicKeysState', () => {
  let IdentityUpdatePublicKeysValidator;
  let DuplicatedIdentityPublicKeyIdError;
  let DuplicatedIdentityPublicKeyError;
  let MaxIdentityPublicKeyLimitReachedError;

  let validatePublicKeys;
  let rawPublicKeys;

  before(async () => {
    ({
      IdentityUpdatePublicKeysValidator,
      DuplicatedIdentityPublicKeyError,
      DuplicatedIdentityPublicKeyIdError,
      MaxIdentityPublicKeyLimitReachedError,
    } = await loadWasmDpp());
  });

  beforeEach(() => {
    ({ publicKeys: rawPublicKeys } = getIdentityFixture().toObject());
    const validator = new IdentityUpdatePublicKeysValidator();
    validatePublicKeys = (keys) => validator.validate(keys);
  });

  it('should return invalid result if there are duplicate key ids', async () => {
    rawPublicKeys[1].id = rawPublicKeys[0].id;

    const result = validatePublicKeys(rawPublicKeys);

    await expectValidationError(result, DuplicatedIdentityPublicKeyIdError);

    const [error] = result.getErrors();

    // TODO: fix? DuplicatedIdentityPublicKeyIdError has error code that comes from
    // consensusError scope, not the one that supposed to match StateError code.

    // expect(error.getCode()).to.equal(4022);
    expect(error.getCode()).to.equal(1030);
    expect(error.getDuplicatedIds()).to.deep.equal([rawPublicKeys[1].id]);
  });

  it('should return invalid result if there are duplicate keys', async () => {
    rawPublicKeys[1].data = rawPublicKeys[0].data;

    const result = validatePublicKeys(rawPublicKeys);

    await expectValidationError(result, DuplicatedIdentityPublicKeyError);

    const [error] = result.getErrors();

    // TODO: fix? DuplicatedIdentityPublicKeyIdError has error code that comes from
    // consensusError scope, not the one that supposed to match StateError code.

    // expect(error.getCode()).to.equal(4021);
    expect(error.getCode()).to.equal(1029);
    expect(error.getDuplicatedPublicKeysIds()).to.deep.equal([rawPublicKeys[1].id]);
  });

  it('should pass valid public keys', () => {
    const result = validatePublicKeys(rawPublicKeys);

    expect(result.isValid()).to.be.true();
  });

  it('should return invalid result if number of public keys is bigger than 32', async () => {
    const { maxItems } = identitySchema.properties.publicKeys;
    const numToAdd = maxItems - rawPublicKeys.length;

    for (let i = 0; i <= numToAdd; ++i) {
      rawPublicKeys.push(rawPublicKeys[0]);
    }

    const result = validatePublicKeys(rawPublicKeys);

    await expectValidationError(result, MaxIdentityPublicKeyLimitReachedError);

    const [error] = result.getErrors();
    expect(error.getCode()).to.equal(4020);
    expect(error.getMaxItems()).to.equal(maxItems);
  });
});
