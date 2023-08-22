const { expectValidationError } = require('../../../lib/test/expect/expectError');

let {
  UnsupportedProtocolVersionError,
  CompatibleProtocolVersionIsNotDefinedError,
  IncompatibleProtocolVersionError,
  ProtocolVersionValidator,
} = require('../../..');
const { default: loadWasmDpp } = require('../../..');

describe.skip('validateProtocolVersionFactory', () => {
  let protocolVersionValidator;
  let versionCompatibilityMap;
  let currentProtocolVersion;
  let protocolVersion;

  before(async () => {
    ({
      UnsupportedProtocolVersionError,
      CompatibleProtocolVersionIsNotDefinedError,
      IncompatibleProtocolVersionError,
      ProtocolVersionValidator,
    } = await loadWasmDpp());
  });

  beforeEach(() => {
    protocolVersion = 1;
    currentProtocolVersion = 1;

    versionCompatibilityMap = {
      1: 1,
    };

    protocolVersionValidator = new ProtocolVersionValidator({
      currentProtocolVersion,
      latestProtocolVersion: protocolVersion,
      versionCompatibilityMap,
    });
  });

  it('should throw UnsupportedProtocolVersionError if protocolVersion is higher than latestVersion', async () => {
    const highVersion = protocolVersion + 1;

    const result = protocolVersionValidator.validate(highVersion);

    await expectValidationError(result, UnsupportedProtocolVersionError);

    const error = result.getFirstError();

    expect(error.getParsedProtocolVersion()).to.equal(highVersion);
    expect(error.getLatestVersion()).to.equal(protocolVersion);
    expect(error.getCode()).to.equal(1002);
  });

  it('should throw CompatibleProtocolVersionIsNotDefinedError if compatible version is not'
    + ' defined for the current protocol version', () => {
    delete versionCompatibilityMap[currentProtocolVersion.toString()];

    protocolVersionValidator = new ProtocolVersionValidator({
      currentProtocolVersion,
      latestProtocolVersion: protocolVersion,
      versionCompatibilityMap,
    });

    try {
      protocolVersionValidator.validate(protocolVersion);

      expect.fail('should throw CompatibleProtocolVersionIsNotDefinedError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(CompatibleProtocolVersionIsNotDefinedError);
    }
  });

  it('should throw IncompatibleProtocolVersionError if parsed version is lower than compatible one', async () => {
    const minimalProtocolVersion = 1;

    protocolVersion = 0;
    currentProtocolVersion = 5;

    versionCompatibilityMap[currentProtocolVersion.toString()] = minimalProtocolVersion;

    protocolVersionValidator = new ProtocolVersionValidator({
      currentProtocolVersion,
      latestProtocolVersion: protocolVersion,
      versionCompatibilityMap,
    });

    const result = protocolVersionValidator.validate(protocolVersion);

    await expectValidationError(result, IncompatibleProtocolVersionError);

    const error = result.getFirstError();

    expect(error.getParsedProtocolVersion()).to.equal(protocolVersion);
    expect(error.getMinimalProtocolVersion()).to.equal(minimalProtocolVersion);
    expect(error.getCode()).to.equal(1003);
  });
});
