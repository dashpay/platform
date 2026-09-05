import fs from 'fs';
import path from 'path';
import HomeDir from '../../../../src/config/HomeDir.js';
import startNodeTaskFactory from '../../../../src/listr/tasks/startNodeTaskFactory.js';

describe('startNodeTaskFactory', () => {
  const configName = 'local';

  let homeDir;
  let config;
  let keyFilePath;
  let startNodeTask;

  beforeEach(function beforeEach() {
    homeDir = HomeDir.createTemp();

    const options = {
      'core.miner.enable': false,
      network: 'testnet',
      'core.log.filePath': null,
      'platform.enable': true,
      'platform.drive.abci.logs': {},
      'platform.gateway.log.accessLogs': [],
      'platform.drive.tenderdash.log.path': null,
      'platform.dapi.rsDapi.logs.accessLogPath': null,
    };

    config = {
      getName: this.sinon.stub().returns(configName),
      get: this.sinon.stub().callsFake((option) => options[option]),
    };

    keyFilePath = homeDir.joinPath(configName, 'platform', 'gateway', 'ssl', 'private.key');

    startNodeTask = startNodeTaskFactory(
      {}, // dockerCompose
      this.sinon.stub(), // waitForCorePeersConnected
      this.sinon.stub(), // waitForMasternodesSync
      this.sinon.stub(), // createRpcClient
      this.sinon.stub(), // buildServicesTask
      this.sinon.stub(), // getConnectionHost
      this.sinon.stub(), // ensureFileMountExists
      homeDir,
      this.sinon.stub().returns([]), // getConfigProfiles
    );
  });

  afterEach(() => {
    homeDir.remove();
  });

  /**
   * @returns {number}
   */
  function getPermissions() {
    // eslint-disable-next-line no-bitwise
    return fs.statSync(keyFilePath).mode & 0o777;
  }

  /**
   * @param {number} mode
   */
  function createPrivateKeyFile(mode) {
    fs.mkdirSync(path.dirname(keyFilePath), { recursive: true });
    fs.writeFileSync(keyFilePath, 'PRIVATE KEY', 'utf8');
    fs.chmodSync(keyFilePath, mode);
  }

  it('should restrict access to a world-readable gateway TLS private key', () => {
    createPrivateKeyFile(0o644);

    startNodeTask(config);

    expect(getPermissions()).to.equal(0o600);
  });

  it('should keep an already restricted private key untouched', () => {
    createPrivateKeyFile(0o600);

    startNodeTask(config);

    expect(getPermissions()).to.equal(0o600);
  });

  // The same rule the certificate writers follow: only the group and world bits
  // are dropped, so an owner who hardened the key further is not undone by a start
  it('should keep a private key mode stricter than Dashmate would choose', () => {
    createPrivateKeyFile(0o400);

    startNodeTask(config);

    expect(getPermissions()).to.equal(0o400);
  });

  it('should not create a private key file if there is none', () => {
    startNodeTask(config);

    // An empty key file would be indistinguishable from a real one for the
    // SSL validation, which decides whether a new certificate has to be obtained
    expect(fs.existsSync(keyFilePath)).to.be.false();
  });

  it('should warn and start anyway if the private key permissions can not be restricted', function it() {
    createPrivateKeyFile(0o644);

    const consoleWarn = this.sinon.stub(console, 'warn');
    this.sinon.stub(fs, 'chmodSync').throws(new Error('EPERM: operation not permitted'));

    expect(() => startNodeTask(config)).to.not.throw();

    expect(consoleWarn).to.be.calledOnce();
  });
});
