const identitySchema = require('../../../../../../../../rs-dpp/src/schema/identity/v0/identity.json');
const { expectValidationError } = require('../../../../../../../lib/test/expect/expectError');

const { default: loadWasmDpp } = require('../../../../../../../dist');

describe.skip('validatePublicKeysState', () => {
  let IdentityUpdatePublicKeysValidator;
  let DuplicatedIdentityPublicKeyIdStateError;
  let DuplicatedIdentityPublicKeyStateError;
  let IdentityPublicKey;
  let MaxIdentityPublicKeyLimitReachedError;

  let validatePublicKeys;
  let rawPublicKeys;

  before(async () => {
    ({
      IdentityPublicKey,
      IdentityUpdatePublicKeysValidator,
      DuplicatedIdentityPublicKeyStateError,
      DuplicatedIdentityPublicKeyIdStateError,
      MaxIdentityPublicKeyLimitReachedError,
    } = await loadWasmDpp());
  });

  beforeEach(() => {
    rawPublicKeys = [
      {
        id: 0,
        type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
        data: Buffer.from('AuryIuMtRrl/VviQuyLD1l4nmxi9ogPzC9LT7tdpo0di', 'base64'),
        purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
        securityLevel: IdentityPublicKey.SECURITY_LEVELS.MASTER,
        signature: Buffer.alloc(0),
        readOnly: false,
      },
      {
        id: 1,
        type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
        data: Buffer.from('A8AK95PYMVX5VQKzOhcVQRCUbc9pyg3RiL7jttEMDU+L', 'base64'),
        purpose: IdentityPublicKey.PURPOSES.ENCRYPTION,
        securityLevel: IdentityPublicKey.SECURITY_LEVELS.MEDIUM,
        signature: Buffer.alloc(0),
        readOnly: false,
      },
    ];

    const validator = new IdentityUpdatePublicKeysValidator();
    validatePublicKeys = (keys) => validator.validate(keys);
  });

  it('should return invalid result if there are duplicate key ids', async () => {
    rawPublicKeys[1].id = rawPublicKeys[0].id;

    const result = validatePublicKeys(rawPublicKeys);

    await expectValidationError(result, DuplicatedIdentityPublicKeyIdStateError);

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(4022);
    expect(error.getDuplicatedIds()).to.deep.equal([rawPublicKeys[1].id]);
  });

  it('should return invalid result if there are duplicate keys', async () => {
    rawPublicKeys[1].data = rawPublicKeys[0].data;

    const result = validatePublicKeys(rawPublicKeys);

    await expectValidationError(result, DuplicatedIdentityPublicKeyStateError);

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(4021);
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
