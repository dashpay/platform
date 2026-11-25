import detectInstallMethod, { INSTALL_METHODS } from '../../../src/update/detectInstallMethod.js';

const DEB_ROOT = '/usr/lib/dashmate';
const DEB_BIN = '/usr/lib/dashmate/bin/dashmate';
const NPM_ROOT = '/usr/local/lib/node_modules/dashmate';
const NVM_ROOT = '/home/dash/.nvm/versions/node/v20.11.0/lib/node_modules/dashmate';
const SOURCE_ROOT = '/home/dash/platform/packages/dashmate';

const DPKG_LIST_OUTPUT = [
  '/usr',
  '/usr/lib',
  '/usr/lib/dashmate',
  '/usr/lib/dashmate/package.json',
  '/usr/lib/dashmate/bin',
  '/usr/lib/dashmate/bin/dashmate',
  '/usr/lib/dashmate/bin/run',
  '/usr/bin/dashmate',
  '',
].join('\n');

describe('detectInstallMethod', () => {
  let runProbe;
  let pathExists;

  /**
   * @param {object} [overrides]
   * @return {Promise<object>}
   */
  function detect(overrides = {}) {
    return detectInstallMethod({
      env: {},
      platform: 'linux',
      packageRootDir: SOURCE_ROOT,
      runProbe,
      pathExists,
      ...overrides,
    });
  }

  beforeEach(function beforeEach() {
    runProbe = this.sinon.stub().rejects(new Error('probe not stubbed'));
    pathExists = this.sinon.stub().resolves(false);
  });

  describe('deb', () => {
    beforeEach(() => {
      runProbe.withArgs('/usr/bin/dpkg-query', ['-S', DEB_ROOT])
        .resolves(`dashmate: ${DEB_ROOT}\n`);
      runProbe.withArgs('/usr/bin/dpkg-query', ['-L', 'dashmate'])
        .resolves(DPKG_LIST_OUTPUT);
    });

    it('should detect a deb install from the package root under /usr/lib/dashmate', async () => {
      const detection = await detect({
        env: { DASHMATE_UPDATE_INSTRUCTIONS: 'update with "sudo apt update && sudo apt install dashmate"' },
        packageRootDir: DEB_ROOT,
      });

      expect(detection.method).to.equal(INSTALL_METHODS.DEB);
      expect(detection.binPath).to.equal(DEB_BIN);
      expect(detection.evidence.debPackageConfirmed).to.be.true();
    });

    it('should detect a deb install when the bin wrapper was bypassed and no env var is set', async () => {
      const detection = await detect({ packageRootDir: DEB_ROOT });

      expect(detection.method).to.equal(INSTALL_METHODS.DEB);
      expect(detection.binPath).to.equal(DEB_BIN);
    });

    it('should fall back to the packaged bin path when dpkg cannot be queried', async () => {
      runProbe = () => Promise.reject(new Error('spawn /usr/bin/dpkg-query ENOENT'));

      const detection = await detect({ packageRootDir: DEB_ROOT });

      expect(detection.method).to.equal(INSTALL_METHODS.DEB);
      expect(detection.binPath).to.equal(DEB_BIN);
      // dpkg could not answer, which is distinct from answering "not ours"
      expect(detection.evidence.debPackageConfirmed).to.be.null();
    });

    it('should return unknown when dpkg reports another package owning the deb root', async () => {
      // Something else was unpacked over the deb's directory, so driving apt at the dashmate
      // package would upgrade something other than what is running
      runProbe.withArgs('/usr/bin/dpkg-query', ['-S', DEB_ROOT])
        .resolves(`dashmate-nightly: ${DEB_ROOT}\n`);

      const detection = await detect({ packageRootDir: DEB_ROOT });

      expect(detection.method).to.equal(INSTALL_METHODS.UNKNOWN);
      expect(detection.evidence.debPackageConfirmed).to.be.false();
    });

    it('should detect a deb install outside the default root when dpkg owns the package root', async () => {
      const relocatedRoot = '/opt/dashmate';

      runProbe.withArgs('/usr/bin/dpkg-query', ['-S', relocatedRoot])
        .resolves(`dashmate: ${relocatedRoot}\n`);

      const detection = await detect({
        env: { DASHMATE_UPDATE_INSTRUCTIONS: 'update with "sudo apt install dashmate"' },
        packageRootDir: relocatedRoot,
      });

      expect(detection.method).to.equal(INSTALL_METHODS.DEB);
      expect(detection.binPath).to.equal(DEB_BIN);
    });

    it('should return unknown when the update instructions env var was inherited from another process', async () => {
      // The wrapper exports the variable, so any child dashmate sees it regardless of how
      // that child was installed. dpkg owning nothing is what tells the two apart.
      runProbe.withArgs('/usr/bin/dpkg-query', ['-S', NPM_ROOT])
        .rejects(new Error('no path found matching pattern'));

      const detection = await detect({
        env: { DASHMATE_UPDATE_INSTRUCTIONS: 'update with "sudo apt install dashmate"' },
        packageRootDir: NPM_ROOT,
      });

      expect(detection.method).to.equal(INSTALL_METHODS.UNKNOWN);
      expect(detection.binPath).to.be.null();
    });

    it('should not detect a deb install on macOS even when the update instructions env var is set', async () => {
      const detection = await detect({
        env: { DASHMATE_UPDATE_INSTRUCTIONS: 'update with "sudo apt install dashmate"' },
        platform: 'darwin',
        packageRootDir: DEB_ROOT,
      });

      expect(detection.method).to.equal(INSTALL_METHODS.UNKNOWN);
      expect(runProbe).to.have.not.been.called();
    });

    it('should not treat a directory that merely shares the deb root prefix as a deb install', async () => {
      // /usr/lib/dashmate-next is a sibling of the deb root, not a child of it
      runProbe.withArgs('/usr/bin/dpkg-query', ['-S', '/usr/lib/dashmate-next'])
        .rejects(new Error('no path found matching pattern'));

      const detection = await detect({
        env: { DASHMATE_UPDATE_INSTRUCTIONS: 'update with "sudo apt install dashmate"' },
        packageRootDir: '/usr/lib/dashmate-next',
      });

      expect(detection.method).to.equal(INSTALL_METHODS.UNKNOWN);
    });

    it('should provide the apt upgrade command', async () => {
      const detection = await detect({ packageRootDir: DEB_ROOT });

      expect(detection.updateInstructions).to.equal('sudo apt update && sudo apt install dashmate');
    });
  });

  describe('tarball', () => {
    it('should detect a tarball install from the bin path env var', async () => {
      const binPath = '/opt/dashmate/bin/dashmate';

      pathExists.withArgs(binPath).resolves(true);

      const detection = await detect({
        env: { DASHMATE_BINPATH: binPath },
        packageRootDir: '/opt/dashmate',
      });

      expect(detection.method).to.equal(INSTALL_METHODS.TARBALL);
      expect(detection.binPath).to.equal(binPath);
    });

    it('should detect a macOS pkg install, which shares the tarball layout', async () => {
      const binPath = '/usr/local/lib/dashmate/bin/dashmate';

      pathExists.withArgs(binPath).resolves(true);

      const detection = await detect({
        env: { DASHMATE_BINPATH: binPath },
        platform: 'darwin',
        packageRootDir: '/usr/local/lib/dashmate',
      });

      expect(detection.method).to.equal(INSTALL_METHODS.TARBALL);
      expect(detection.binPath).to.equal(binPath);
    });

    it('should not return a relative bin path, which cannot be re-executed safely', async () => {
      const detection = await detect({
        env: { DASHMATE_BINPATH: './bin/dashmate' },
        packageRootDir: '/opt/dashmate',
      });

      expect(detection.method).to.equal(INSTALL_METHODS.TARBALL);
      expect(detection.binPath).to.be.null();
    });

    it('should not return a bin path that does not exist', async () => {
      pathExists.resolves(false);

      const detection = await detect({
        env: { DASHMATE_BINPATH: '/opt/dashmate/bin/dashmate' },
        packageRootDir: '/opt/dashmate',
      });

      expect(detection.method).to.equal(INSTALL_METHODS.TARBALL);
      expect(detection.binPath).to.be.null();
    });

    it('should tell the operator to download a release, since we cannot drive the upgrade', async () => {
      const detection = await detect({
        env: { DASHMATE_BINPATH: '/opt/dashmate/bin/dashmate' },
        packageRootDir: '/opt/dashmate',
      });

      expect(detection.updateInstructions).to.include('https://github.com/dashpay/platform/releases/latest');
    });
  });

  describe('npm', () => {
    it('should detect a global npm install and resolve the bin shim', async () => {
      pathExists.withArgs('/usr/local/bin/dashmate').resolves(true);

      const detection = await detect({ packageRootDir: NPM_ROOT });

      expect(detection.method).to.equal(INSTALL_METHODS.NPM);
      expect(detection.binPath).to.equal('/usr/local/bin/dashmate');
      expect(detection.evidence.npmGlobalPrefix).to.equal('/usr/local');
    });

    it('should detect an npm install under an nvm node version', async () => {
      const nvmBin = '/home/dash/.nvm/versions/node/v20.11.0/bin/dashmate';

      pathExists.withArgs(nvmBin).resolves(true);

      const detection = await detect({ packageRootDir: NVM_ROOT });

      expect(detection.method).to.equal(INSTALL_METHODS.NPM);
      expect(detection.binPath).to.equal(nvmBin);
    });

    it('should detect an npm install without a bin path when the shim is missing', async () => {
      pathExists.resolves(false);

      const detection = await detect({ packageRootDir: NPM_ROOT });

      expect(detection.method).to.equal(INSTALL_METHODS.NPM);
      expect(detection.binPath).to.be.null();
    });

    it('should detect an npm install with no bin path when the prefix layout is not npm global', async () => {
      // yarn's global folder has no lib/ level, so the bin directory cannot be derived
      const detection = await detect({
        packageRootDir: '/home/dash/.config/yarn/global/node_modules/dashmate',
      });

      expect(detection.method).to.equal(INSTALL_METHODS.NPM);
      expect(detection.binPath).to.be.null();
      expect(detection.evidence.npmGlobalPrefix).to.be.null();
    });

    it('should provide the npm upgrade command', async () => {
      const detection = await detect({ packageRootDir: NPM_ROOT });

      expect(detection.updateInstructions).to.equal('npm install -g dashmate@latest');
    });

    it('should not run any probe, since the layout answers the question', async () => {
      await detect({ packageRootDir: NPM_ROOT });

      expect(runProbe).to.have.not.been.called();
    });
  });

  describe('unknown', () => {
    it('should return unknown for a source checkout', async () => {
      const detection = await detect({ packageRootDir: SOURCE_ROOT });

      expect(detection.method).to.equal(INSTALL_METHODS.UNKNOWN);
      expect(detection.binPath).to.be.null();
      expect(detection.updateInstructions).to.be.a('string').and.not.empty();
    });

    it('should return unknown when both bin wrapper env vars are set, which no wrapper does', async () => {
      const detection = await detect({
        env: {
          DASHMATE_BINPATH: '/opt/dashmate/bin/dashmate',
          DASHMATE_UPDATE_INSTRUCTIONS: 'update with "sudo apt install dashmate"',
        },
        packageRootDir: '/opt/dashmate',
      });

      expect(detection.method).to.equal(INSTALL_METHODS.UNKNOWN);
      expect(detection.binPath).to.be.null();
    });

    it('should ignore empty env vars rather than treating them as signals', async () => {
      const detection = await detect({
        env: { DASHMATE_BINPATH: '', DASHMATE_UPDATE_INSTRUCTIONS: '' },
        packageRootDir: SOURCE_ROOT,
      });

      expect(detection.method).to.equal(INSTALL_METHODS.UNKNOWN);
      expect(detection.evidence.hasBinPathEnv).to.be.false();
      expect(detection.evidence.hasUpdateInstructionsEnv).to.be.false();
    });
  });

  describe('windows', () => {
    it('should degrade to unknown without probing, since Windows has no dpkg', async () => {
      const detection = await detect({
        env: { DASHMATE_BINPATH: 'C:\\Users\\dash\\AppData\\Local\\dashmate\\bin\\dashmate.cmd' },
        platform: 'win32',
        packageRootDir: 'C:\\Users\\dash\\AppData\\Local\\dashmate',
      });

      expect(detection.method).to.equal(INSTALL_METHODS.UNKNOWN);
      expect(detection.binPath).to.be.null();
      expect(detection.updateInstructions).to.be.a('string').and.not.empty();
      expect(runProbe).to.have.not.been.called();
      expect(pathExists).to.have.not.been.called();
    });
  });

  describe('never throws', () => {
    it('should return unknown instead of throwing when a probe rejects', async () => {
      runProbe.rejects(new Error('dpkg database is locked'));

      const detection = await detect({
        env: { DASHMATE_UPDATE_INSTRUCTIONS: 'update with "sudo apt install dashmate"' },
        packageRootDir: '/opt/dashmate',
      });

      expect(detection.method).to.equal(INSTALL_METHODS.UNKNOWN);
    });

    it('should return unknown instead of throwing when a filesystem check rejects', async () => {
      pathExists.rejects(new Error('EACCES'));

      const detection = await detect({
        env: { DASHMATE_BINPATH: '/opt/dashmate/bin/dashmate' },
        packageRootDir: '/opt/dashmate',
      });

      expect(detection.method).to.equal(INSTALL_METHODS.UNKNOWN);
      expect(detection.evidence.reason).to.include('EACCES');
    });

    it('should return unknown instead of throwing when the package root is not a path', async () => {
      const detection = await detect({ packageRootDir: null });

      expect(detection.method).to.equal(INSTALL_METHODS.UNKNOWN);
    });
  });

  describe('subprocess discipline', () => {
    it('should probe dpkg by absolute path with an argv array', async () => {
      runProbe.resolves(`dashmate: ${DEB_ROOT}\n`);

      await detect({ packageRootDir: DEB_ROOT });

      expect(runProbe).to.have.been.called();

      runProbe.getCalls().forEach((call) => {
        expect(call.args[0]).to.equal('/usr/bin/dpkg-query');
        expect(call.args[1]).to.be.an('array');
      });
    });
  });
});
