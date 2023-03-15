const createDPPMock = require('@dashevo/dpp/lib/test/mocks/createDPPMock');

const { expectValidationError } = require('@dashevo/dpp/lib/test/expect/expectError');
const { latestVersion } = require('@dashevo/dpp/lib/version/protocolVersion');

let {
  UnsupportedProtocolVersionError,
  CompatibleProtocolVersionIsNotDefinedError,
  IncompatibleProtocolVersionError,
  ProtocolVersionValidator,
} = require('../../..');
const { default: loadWasmDpp } = require('../../..');

describe('validateProtocolVersionFactory', () => {
  let validateProtocolVersion;
  let dppMock;
  let versionCompatibilityMap;
  let currentProtocolVersion;
  let protocolVersion;

  beforeEach(async function beforeEach() {
    ({
      UnsupportedProtocolVersionError,
      CompatibleProtocolVersionIsNotDefinedError,
      IncompatibleProtocolVersionError,
      ProtocolVersionValidator,
    } = await loadWasmDpp());

    protocolVersion = 1;
    currentProtocolVersion = 1;

    dppMock = createDPPMock(this.sinonSandbox);
    dppMock.getProtocolVersion.returns(currentProtocolVersion);

    versionCompatibilityMap = {
      1: 1,
    };

    validateProtocolVersion = new ProtocolVersionValidator({
      currentProtocolVersion,
      latestProtocolVersion: protocolVersion,
      versionCompatibilityMap,
    });
  });

  it('should throw UnsupportedProtocolVersionError if protocolVersion is higher than latestVersion', () => {
    protocolVersion = latestVersion + 1;

    const result = validateProtocolVersion(protocolVersion);

    expectValidationError(result, UnsupportedProtocolVersionError);

    const error = result.getFirstError();

    expect(error.getParsedProtocolVersion()).to.equal(protocolVersion);
    expect(error.getLatestVersion()).to.equal(latestVersion);
    expect(error.getCode()).to.equal(1002);
  });

  it('should throw CompatibleProtocolVersionIsNotDefinedError if compatible version is not'
    + ' defined for the current protocol version', () => {
    delete versionCompatibilityMap[currentProtocolVersion.toString()];

    try {
      validateProtocolVersion(protocolVersion);

      expect.fail('should throw CompatibleProtocolVersionIsNotDefinedError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(CompatibleProtocolVersionIsNotDefinedError);
    }
  });

  it('should throw IncompatibleProtocolVersionError if parsed version is lower than compatible one', () => {
    const minimalProtocolVersion = 1;

    protocolVersion = 0;
    currentProtocolVersion = 5;

    versionCompatibilityMap[currentProtocolVersion.toString()] = minimalProtocolVersion;

    const result = validateProtocolVersion(protocolVersion);

    expectValidationError(result, IncompatibleProtocolVersionError);

    const error = result.getFirstError();

    expect(error.getParsedProtocolVersion()).to.equal(protocolVersion);
    expect(error.getMinimalProtocolVersion()).to.equal(minimalProtocolVersion);
    expect(error.getCode()).to.equal(1003);
  });
});
