const getIdentityCreateTransitionFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityCreateTransitionFixture');

const createStateRepositoryMock = require('@dashevo/dpp/lib/test/mocks/createStateRepositoryMock');

const protocolVersion = require('@dashevo/dpp/lib/version/protocolVersion');
const { default: loadWasmDpp } = require('../../../../../../../dist');
const generateRandomIdentifierAsync = require('../../../../../../../lib/test/utils/generateRandomIdentifierAsync');
const { expectValidationError } = require('../../../../../../../lib/test/expect/expectError');

describe('validateIdentityCreateTransitionStateFactory', () => {
  let validateIdentityCreateTransitionState;
  let stateTransition;
  let stateRepositoryMock;
  let identity;

  let IdentityCreateTransition;
  let IdentityPublicKey;
  let IdentityAlreadyExistsError;
  let Identity;
  let IdentityCreateTransitionStateValidator;

  before(async () => {
    ({
      IdentityCreateTransition,
      IdentityAlreadyExistsError,
      IdentityPublicKey,
      Identity,
      IdentityCreateTransitionStateValidator,
    } = await loadWasmDpp());
  });

  beforeEach(async function () {
    const privateKey = Buffer.from('af432c476f65211f45f48f1d42c9c0b497e56696aa1736b40544ef1a496af837', 'hex');
    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);

    const validator = new IdentityCreateTransitionStateValidator(stateRepositoryMock);
    validateIdentityCreateTransitionState = (st) => validator.validate(st);

    stateTransition = new IdentityCreateTransition(
      getIdentityCreateTransitionFixture().toObject(),
    );

    await stateTransition.signByPrivateKey(privateKey, IdentityPublicKey.TYPES.ECDSA_SECP256K1);

    const rawTransaction = '030000000137feb5676d0851337ea3c9a992496aab7a0b3eee60aeeb9774000b7f4bababa5000000006b483045022100d91557de37645c641b948c6cd03b4ae3791a63a650db3e2fee1dcf5185d1b10402200e8bd410bf516ca61715867666d31e44495428ce5c1090bf2294a829ebcfa4ef0121025c3cc7fbfc52f710c941497fd01876c189171ea227458f501afcb38a297d65b4ffffffff021027000000000000166a14152073ca2300a86b510fa2f123d3ea7da3af68dcf77cb0090a0000001976a914152073ca2300a86b510fa2f123d3ea7da3af68dc88ac00000000';

    stateRepositoryMock.fetchTransaction.returns({
      data: Buffer.from(rawTransaction, 'hex'),
      height: 42,
    });

    identity = new Identity({
      protocolVersion: protocolVersion.latestVersion,
      id: await generateRandomIdentifierAsync(),
      publicKeys: [
        {
          id: 0,
          type: 0,
          data: Buffer.alloc(36).fill('a'),
          purpose: 0,
          securityLevel: 0,
          readOnly: false,
          signature: Buffer.alloc(36).fill('a'),
        },
      ],
      balance: 0,
      revision: 0,
    });
  });

  it('should return invalid result if identity already exists', async () => {
    stateRepositoryMock.fetchIdentity.returns(identity);

    const result = await validateIdentityCreateTransitionState(stateTransition);

    await expectValidationError(result, IdentityAlreadyExistsError);

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(4011);
    expect(error.getIdentityId()).to.exist();
    expect(error.getIdentityId()).to.deep.equal(stateTransition.getIdentityId().toBuffer());
  });

  it('should return valid result if state transition is valid', async () => {
    const result = await validateIdentityCreateTransitionState(stateTransition);

    expect(result.isValid()).to.be.true();
  });

  it('should return valid result on dry run', async () => {
    stateRepositoryMock.fetchIdentity.returns(identity);

    stateTransition.getExecutionContext().enableDryRun();

    const result = await validateIdentityCreateTransitionState(stateTransition);

    stateTransition.getExecutionContext().disableDryRun();

    expect(result.isValid()).to.be.true();
  });
});
