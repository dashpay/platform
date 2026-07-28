import { promises as fsPromises } from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

import { INSTALL_METHODS } from '../../../src/update/detectInstallMethod.js';
import upgradePackage, {
  assertArgv,
  ensureRootAccess,
  isRunningAsRoot,
  PackageUpgradeError,
  PACKAGE_STATE,
  ROOT_ACCESS,
  RUNNING_AS_ROOT_WARNING,
  UPGRADE_STEP,
} from '../../../src/update/upgradePackage.js';

const MODULE_PATH = fileURLToPath(new URL('../../../src/update/upgradePackage.js', import.meta.url));

const SUDO = '/usr/bin/sudo';
const ENV = '/usr/bin/env';
const APT_GET = '/usr/bin/apt-get';
const DPKG = '/usr/bin/dpkg';
const DPKG_QUERY = '/usr/bin/dpkg-query';

const DEB_BIN_PATH = '/usr/lib/dashmate/bin/dashmate';
const NPM_PREFIX = '/home/dash/.nvm/versions/node/v20.11.0';
const NPM_PATH = `${NPM_PREFIX}/bin/npm`;
const NPM_BIN_PATH = `${NPM_PREFIX}/bin/dashmate`;
const NPM_MANIFEST_PATH = `${NPM_PREFIX}/lib/node_modules/dashmate/package.json`;
const NODE_EXEC_PATH = `${NPM_PREFIX}/bin/node`;

const VERSION = '4.2.0';
const DEB_VERSION = '4.2.0-1';

/**
 * The complete argv of the privileged apt invocation, written out rather than derived.
 *
 * This is the pin: a deleted verb, a dropped option, a reordered pair or an extra flag all fail
 * here, which targeted `include` assertions cannot catch.
 */
const APT_ARGS = [
  'install',
  '--only-upgrade',
  '--no-remove',
  '-y',
  '-o', 'Dir::Etc::sourcelist=/etc/apt/sources.list.d/dashmate.sources',
  '-o', 'Dir::Etc::sourceparts=/dev/null',
  '-o', 'DPkg::Lock::Timeout=600',
  '-o', 'Dpkg::Options::=--force-confold',
  `dashmate=${DEB_VERSION}`,
];

const APT_ENV_ARGS = [
  ENV,
  '-i',
  'PATH=/usr/sbin:/usr/bin:/sbin:/bin',
  'HOME=/root',
  'LANG=C',
  'LC_ALL=C',
  'DEBIAN_FRONTEND=noninteractive',
  APT_GET,
  ...APT_ARGS,
];

const SUDO_APT_ARGV = ['-n', '--', ...APT_ENV_ARGS];
const SUDO_PREFLIGHT_ARGV = ['-n', '-l', '--', ...APT_ENV_ARGS];

/**
 * An environment of the kind a real operator shell hands us: the variables the module must pass
 * through, and the ones it must never let reach a package manager.
 */
const HOSTILE_ENV = {
  HOME: '/home/dash',
  LANG: 'ru_RU.UTF-8',
  PATH: '/tmp/evil:/usr/bin',
  APT_CONFIG: '/tmp/evil/apt.conf',
  NODE_OPTIONS: '--require /tmp/evil/preload.js',
  NODE_PATH: '/tmp/evil/modules',
  LD_PRELOAD: '/tmp/evil/lib.so',
  LD_LIBRARY_PATH: '/tmp/evil',
  DEBIAN_FRONTEND: 'readline',
};

const SUDOERS_REFUSAL = 'Sorry, user dash is not allowed to execute \'/usr/bin/env -i ...\' as root on node1.';

/**
 * @param {Object} [overrides]
 * @return {Object}
 */
function commandResult(overrides = {}) {
  return {
    exitCode: 0, stdout: '', stderr: '', signal: null, spawnError: null, ...overrides,
  };
}

describe('upgradePackage', () => {
  let runCommand;
  let runInteractiveCommand;
  let pathExists;
  let readFile;

  /**
   * Every argv the module handed to a subprocess seam, in order.
   *
   * @return {Array<{file: string, args: string[], options: Object}>}
   */
  function invocations() {
    return [...runCommand.getCalls(), ...runInteractiveCommand.getCalls()]
      .map((call) => ({ file: call.args[0], args: call.args[1], options: call.args[2] }));
  }

  /**
   * @param {string} file
   * @return {Array<{file: string, args: string[], options: Object}>}
   */
  function invocationsOf(file) {
    return invocations().filter((invocation) => invocation.file === file);
  }

  /**
   * @param {Object} [overrides]
   * @return {Promise<Object>}
   */
  function upgrade(overrides = {}) {
    return upgradePackage({
      installMethod: INSTALL_METHODS.DEB,
      version: VERSION,
      expectedDebVersion: DEB_VERSION,
      binPath: DEB_BIN_PATH,
      platform: 'linux',
      euid: 1000,
      env: HOSTILE_ENV,
      nodeExecPath: NODE_EXEC_PATH,
      rootAccess: {
        elevation: ROOT_ACCESS.SUDO, euid: 1000, authenticated: true, warnings: [],
      },
      runCommand,
      runInteractiveCommand,
      pathExists,
      readFile,
      ...overrides,
    });
  }

  /**
   * A host where every deb step answers the way it does on a healthy upgrade.
   *
   * @param {Object} [options]
   */
  function stubHealthyDebHost({ before = '4.1.0-1', after = DEB_VERSION } = {}) {
    runCommand.withArgs(DPKG, ['--audit']).resolves(commandResult());

    runCommand.withArgs(DPKG_QUERY)
      .onFirstCall().resolves(commandResult({ stdout: before }))
      .onSecondCall()
      .resolves(commandResult({ stdout: after }));

    runCommand.withArgs(SUDO).resolves(commandResult({ stdout: '1 upgraded, 0 newly installed.' }));
  }

  /**
   * A host where apt refused, so the failure path runs.
   *
   * @param {Object} aptResult
   */
  function stubFailingApt(aptResult) {
    runCommand.withArgs(DPKG, ['--audit']).resolves(commandResult());
    runCommand.withArgs(DPKG_QUERY).resolves(commandResult({ stdout: '4.1.0-1' }));
    runCommand.withArgs(SUDO).resolves(commandResult(aptResult));
    runCommand.withArgs(DEB_BIN_PATH).resolves(commandResult({ stdout: 'dashmate/4.1.0 linux-x64' }));
  }

  beforeEach(function beforeEach() {
    runCommand = this.sinon.stub().resolves(commandResult());
    runInteractiveCommand = this.sinon.stub().resolves(commandResult());
    pathExists = this.sinon.stub().resolves(false);
    readFile = this.sinon.stub().rejects(Object.assign(new Error('ENOENT'), { code: 'ENOENT' }));
  });

  describe('deb', () => {
    it('should upgrade the package and assert the version dpkg reports afterwards', async () => {
      stubHealthyDebHost();

      const result = await upgrade();

      expect(result.method).to.equal(INSTALL_METHODS.DEB);
      expect(result.packageState).to.equal(PACKAGE_STATE.UPGRADED);
      expect(result.previousVersion).to.equal('4.1.0-1');
      expect(result.installedVersion).to.equal(DEB_VERSION);
      expect(result.versionScheme).to.equal('debian');
      expect(result.verifiedBy).to.equal('dpkg-query');
      expect(result.elevation).to.equal(ROOT_ACCESS.SUDO);
    });

    it('should invoke apt with exactly the argv this design calls for and nothing else', async () => {
      stubHealthyDebHost();

      await upgrade();

      const [apt] = invocationsOf(SUDO);

      expect(apt.args).to.deep.equal(SUDO_APT_ARGV);
    });

    it('should invoke apt directly, with the same argv, when it is already root', async () => {
      stubHealthyDebHost();
      runCommand.withArgs(ENV).resolves(commandResult());

      await upgrade({
        euid: 0,
        rootAccess: {
          elevation: ROOT_ACCESS.ALREADY_ROOT, euid: 0, authenticated: true, warnings: [],
        },
      });

      expect(invocationsOf(SUDO)).to.have.lengthOf(0);

      const [apt] = invocationsOf(ENV);

      // env still rebuilds the environment: sudo is not the only way something can be inherited
      expect([ENV, ...apt.args]).to.deep.equal(APT_ENV_ARGS.map(
        (arg) => (arg === 'HOME=/root' ? 'HOME=/home/dash' : arg),
      ));
    });

    it('should audit dpkg before it touches the package', async () => {
      stubHealthyDebHost();

      await upgrade();

      expect(runCommand.getCall(0).args[0]).to.equal(DPKG);
      expect(runCommand.getCall(0).args[1]).to.deep.equal(['--audit']);
    });

    it('should let apt wait for the dpkg lock rather than fail while unattended-upgrades holds it', async () => {
      stubHealthyDebHost();

      await upgrade({ lockTimeoutSeconds: 900 });

      const [apt] = invocationsOf(SUDO);

      expect(apt.args).to.include('DPkg::Lock::Timeout=900');
    });

    it('should never weaken apt signature verification or allow a downgrade', async () => {
      stubHealthyDebHost();

      await upgrade();

      const forbidden = [
        '--allow-unauthenticated',
        '--force-yes',
        '--allow-downgrades',
        '--allow-insecure-repositories',
        'APT::Get::AllowUnauthenticated=1',
      ];

      invocations().forEach((invocation) => {
        forbidden.forEach((flag) => {
          expect(invocation.args).to.not.include(flag);
        });
      });
    });

    it('should fail when apt exits 0 but the installed version did not change', async () => {
      // "0 upgraded, 0 newly installed" exits 0 as readily as a real upgrade does
      stubHealthyDebHost({ before: DEB_VERSION, after: DEB_VERSION });
      runCommand.withArgs(DEB_BIN_PATH).resolves(commandResult({ stdout: 'dashmate/4.2.0 linux-x64' }));

      const error = await expect(upgrade()).to.be.rejectedWith(PackageUpgradeError);

      expect(error.step).to.equal(UPGRADE_STEP.VERIFY_INSTALLED_VERSION);
      expect(error.packageState).to.equal(PACKAGE_STATE.INTACT);
      expect(error.message).to.include('nothing was upgraded');
      expect(error.instructions).to.include('sudo apt-get update');
    });

    it('should report a package replaced with a version nobody approved as its own state', async () => {
      // The version is pinned in the argv, so apt should refuse - but if this is ever reached the
      // binary really has been replaced, and the node needs activating rather than reassuring
      stubHealthyDebHost({ before: '4.1.0-1', after: '4.2.0~rc.9-1' });
      runCommand.withArgs(DEB_BIN_PATH).resolves(commandResult({ stdout: 'dashmate/4.2.0-rc.9' }));

      const error = await expect(upgrade()).to.be.rejectedWith(PackageUpgradeError);

      expect(error.step).to.equal(UPGRADE_STEP.VERIFY_INSTALLED_VERSION);
      expect(error.packageState).to.equal(PACKAGE_STATE.UNAPPROVED);
      expect(error.installedVersion).to.equal('4.2.0~rc.9-1');
      expect(error.message).to.not.include('still runs');
    });

    it('should pin the approved Debian version in the argv so apt refuses a release candidate up front', async () => {
      stubHealthyDebHost({ before: '4.1.0-1', after: '4.2.0~rc.3-1' });

      const result = await upgrade({
        version: '4.2.0-rc.3',
        expectedDebVersion: '4.2.0~rc.3-1',
      });

      const [apt] = invocationsOf(SUDO);

      expect(apt.args[apt.args.length - 1]).to.equal('dashmate=4.2.0~rc.3-1');
      expect(result.installedVersion).to.equal('4.2.0~rc.3-1');
    });

    it('should report a pre-broken dpkg state as such instead of attempting the upgrade', async () => {
      runCommand.withArgs(DPKG, ['--audit']).resolves(commandResult({
        stdout: 'The following packages are only half configured:\n dash-core',
      }));

      const error = await expect(upgrade()).to.be.rejectedWith(PackageUpgradeError);

      expect(error.step).to.equal(UPGRADE_STEP.DPKG_AUDIT);
      expect(error.packageState).to.equal(PACKAGE_STATE.UNCHANGED);
      expect(error.message).to.include('dash-core');
      expect(error.instructions).to.include('sudo dpkg --configure -a');
      expect(invocationsOf(SUDO)).to.have.lengthOf(0);
      expect(invocationsOf(APT_GET)).to.have.lengthOf(0);
    });

    it('should report a broken dpkg state that is signalled by an exit code alone', async () => {
      runCommand.withArgs(DPKG, ['--audit']).resolves(commandResult({ exitCode: 2 }));

      const error = await expect(upgrade()).to.be.rejectedWith(PackageUpgradeError);

      expect(error.step).to.equal(UPGRADE_STEP.DPKG_AUDIT);
      expect(invocationsOf(SUDO)).to.have.lengthOf(0);
    });

    it('should name the held apt lock when apt gives up waiting for it', async () => {
      stubFailingApt({
        exitCode: 100,
        stderr: 'E: Could not get lock /var/lib/dpkg/lock-frontend. It is held by process 4242 (unattended-upgr)',
      });

      const error = await expect(upgrade()).to.be.rejectedWith(PackageUpgradeError);

      expect(error.step).to.equal(UPGRADE_STEP.APT_INSTALL);
      expect(error.exitCode).to.equal(100);
      expect(error.message).to.include('unattended-upgrades');
      expect(error.message).to.include('Could not get lock');
      // dashmate itself was verified to still run, so this is a failed upgrade, not a broken host
      expect(error.packageState).to.equal(PACKAGE_STATE.INTACT);
    });

    it('should name the scoping when apt cannot resolve a dependency from our sources alone', async () => {
      stubFailingApt({
        exitCode: 100,
        stderr: 'The following packages have unmet dependencies:\n dashmate : Depends: libssl3 but it is not installable',
      });

      const error = await expect(upgrade()).to.be.rejectedWith(PackageUpgradeError);

      expect(error.message).to.include('scoped to the dashmate repository');
      expect(error.instructions).to.include('sudo apt-get install --only-upgrade dashmate=4.2.0-1');
    });

    it('should tell the operator to fix sudoers, not their password, when sudo refuses the command', async () => {
      stubFailingApt({ exitCode: 1, stderr: SUDOERS_REFUSAL });

      const error = await expect(upgrade()).to.be.rejectedWith(PackageUpgradeError);

      expect(error.message).to.include('sudoers policy');
      // the rule has to name env, because that is what sudo is asked to run
      expect(error.instructions.join('\n')).to.include(`Add to sudoers: <user> ALL=(root) NOPASSWD: ${ENV} -i`);
    });

    it('should report the package state as uncertain when dashmate no longer runs after apt failed', async () => {
      stubFailingApt({
        exitCode: 100,
        stderr: 'dpkg: error processing archive /var/cache/apt/archives/dashmate_4.2.0-1_amd64.deb',
      });
      runCommand.withArgs(DEB_BIN_PATH).resolves(commandResult({ exitCode: 127 }));

      const error = await expect(upgrade()).to.be.rejectedWith(PackageUpgradeError);

      expect(error.packageState).to.equal(PACKAGE_STATE.UNCERTAIN);
      expect(error.instructions).to.include('sudo dpkg --configure -a');
      expect(error.instructions).to.include('sudo apt-get install --reinstall dashmate');
    });

    it('should not claim the package may be broken when there was no executable to check', async () => {
      stubFailingApt({ exitCode: 100, stderr: 'E: Unable to fetch some archives' });

      const error = await expect(upgrade({ binPath: null })).to.be.rejectedWith(PackageUpgradeError);

      expect(error.packageState).to.equal(PACKAGE_STATE.UNVERIFIED);
      expect(error.message).to.include('was not checked');
      expect(error.instructions).to.include('dashmate --version');
      expect(invocationsOf(DEB_BIN_PATH)).to.have.lengthOf(0);
    });

    it('should refuse to upgrade a package dpkg does not report as installed', async () => {
      runCommand.withArgs(DPKG, ['--audit']).resolves(commandResult());
      runCommand.withArgs(DPKG_QUERY).resolves(commandResult({
        exitCode: 1,
        stderr: 'dpkg-query: no packages found matching dashmate',
      }));

      const error = await expect(upgrade()).to.be.rejectedWith(PackageUpgradeError);

      expect(error.step).to.equal(UPGRADE_STEP.QUERY_INSTALLED_VERSION);
      expect(error.packageState).to.equal(PACKAGE_STATE.UNCHANGED);
      expect(invocationsOf(SUDO)).to.have.lengthOf(0);
    });

    it('should refuse a deb upgrade on a platform that has no dpkg', async () => {
      const error = await expect(upgrade({ platform: 'darwin' })).to.be.rejectedWith(PackageUpgradeError);

      expect(error.step).to.equal(UPGRADE_STEP.VALIDATE_INPUT);
      expect(runCommand).to.have.not.been.called();
    });
  });

  describe('npm', () => {
    /**
     * @param {Object} [overrides]
     * @return {Promise<Object>}
     */
    function upgradeNpm(overrides = {}) {
      return upgrade({
        installMethod: INSTALL_METHODS.NPM,
        expectedDebVersion: null,
        binPath: NPM_BIN_PATH,
        npmGlobalPrefix: NPM_PREFIX,
        rootAccess: null,
        ...overrides,
      });
    }

    beforeEach(() => {
      pathExists.withArgs(NPM_PATH).resolves(true);
    });

    it('should install the requested version globally and read the installed version back', async () => {
      readFile.withArgs(NPM_MANIFEST_PATH).resolves(JSON.stringify({ name: 'dashmate', version: VERSION }));

      const result = await upgradeNpm();

      expect(result.method).to.equal(INSTALL_METHODS.NPM);
      expect(result.packageState).to.equal(PACKAGE_STATE.UPGRADED);
      expect(result.installedVersion).to.equal(VERSION);
      expect(result.versionScheme).to.equal('semver');
      expect(result.elevation).to.equal(ROOT_ACCESS.NONE);

      const [npm] = invocationsOf(NPM_PATH);

      expect(npm.args).to.deep.equal(['install', '-g', `dashmate@${VERSION}`]);
    });

    it('should verify the upgrade by running the installed binary when the manifest cannot be read', async () => {
      runCommand.withArgs(NPM_BIN_PATH).resolves(commandResult({
        stdout: 'dashmate/4.2.0 linux-x64 node-v20.11.0',
      }));

      const result = await upgradeNpm();

      expect(result.installedVersion).to.equal(VERSION);
      expect(result.verifiedBy).to.equal(`${NPM_BIN_PATH} --version`);
    });

    it('should report an npm upgrade performed as root, which the rest of the update must not continue from', async () => {
      readFile.withArgs(NPM_MANIFEST_PATH).resolves(JSON.stringify({ version: VERSION }));

      const result = await upgradeNpm({ euid: 0 });

      expect(result.elevation).to.equal(ROOT_ACCESS.ALREADY_ROOT);
      expect(result.warnings).to.include(RUNNING_AS_ROOT_WARNING);
      expect(result.warnings.join(' ')).to.include('/root/.dashmate');
    });

    it('should fail when npm exits 0 but the installed version is not the one requested', async () => {
      readFile.withArgs(NPM_MANIFEST_PATH).resolves(JSON.stringify({ version: '4.1.0' }));

      const error = await expect(upgradeNpm()).to.be.rejectedWith(PackageUpgradeError);

      expect(error.step).to.equal(UPGRADE_STEP.VERIFY_INSTALLED_VERSION);
      expect(error.packageState).to.equal(PACKAGE_STATE.UNAPPROVED);
      expect(error.installedVersion).to.equal('4.1.0');
    });

    it('should not report an upgrade it could not verify as done, and not call it broken either', async () => {
      const error = await expect(upgradeNpm({ npmGlobalPrefix: null, binPath: null }))
        .to.be.rejectedWith(PackageUpgradeError);

      expect(error.step).to.equal(UPGRADE_STEP.VERIFY_INSTALLED_VERSION);
      expect(error.packageState).to.equal(PACKAGE_STATE.UNVERIFIED);
      expect(error.message).to.include('probably did happen');
    });

    it('should print the sudo form of the command when npm cannot write to its global prefix', async () => {
      runCommand.withArgs(NPM_PATH).resolves(commandResult({
        exitCode: 243,
        stderr: 'npm ERR! code EACCES\nnpm ERR! syscall mkdir\nnpm ERR! path /usr/lib/node_modules/dashmate',
      }));
      runCommand.withArgs(NPM_BIN_PATH).resolves(commandResult({ stdout: 'dashmate/4.1.0 linux-x64' }));

      const error = await expect(upgradeNpm()).to.be.rejectedWith(PackageUpgradeError);

      expect(error.step).to.equal(UPGRADE_STEP.NPM_INSTALL);
      expect(error.instructions).to.deep.equal([`sudo npm install -g dashmate@${VERSION}`]);
      expect(error.message).to.include(`sudo npm install -g dashmate@${VERSION}`);
      // an operator whose node came from a version manager must not be told to use sudo blindly
      expect(error.message).to.include('nvm');
      expect(error.packageState).to.equal(PACKAGE_STATE.INTACT);
    });

    it('should recognise EPERM as the same permission failure as EACCES', async () => {
      runCommand.withArgs(NPM_PATH).resolves(commandResult({
        exitCode: 243,
        stderr: 'npm ERR! code EPERM\nnpm ERR! syscall rename',
      }));
      runCommand.withArgs(NPM_BIN_PATH).resolves(commandResult({ stdout: 'dashmate/4.1.0 linux-x64' }));

      const error = await expect(upgradeNpm()).to.be.rejectedWith(PackageUpgradeError);

      expect(error.instructions).to.deep.equal([`sudo npm install -g dashmate@${VERSION}`]);
    });

    it('should print the plain form of the command when npm failed for another reason', async () => {
      runCommand.withArgs(NPM_PATH).resolves(commandResult({
        exitCode: 1,
        stderr: 'npm ERR! code ETARGET\nnpm ERR! notarget No matching version found for dashmate@4.2.0',
      }));
      runCommand.withArgs(NPM_BIN_PATH).resolves(commandResult({ stdout: 'dashmate/4.1.0 linux-x64' }));

      const error = await expect(upgradeNpm()).to.be.rejectedWith(PackageUpgradeError);

      expect(error.instructions).to.deep.equal([`npm install -g dashmate@${VERSION}`]);
      expect(error.message).to.include('No matching version found');
    });

    it('should not repeat the same command twice in what it tells the operator to run', async () => {
      runCommand.withArgs(NPM_PATH).resolves(commandResult({ exitCode: 1, stderr: 'npm ERR! network timeout' }));
      runCommand.withArgs(NPM_BIN_PATH).resolves(commandResult({ exitCode: 127 }));

      const error = await expect(upgradeNpm()).to.be.rejectedWith(PackageUpgradeError);

      expect(error.packageState).to.equal(PACKAGE_STATE.UNCERTAIN);
      expect(error.instructions).to.deep.equal([`npm install -g dashmate@${VERSION}`]);
    });

    it('should never retry a failed global install', async () => {
      runCommand.withArgs(NPM_PATH).resolves(commandResult({ exitCode: 1, stderr: 'npm ERR! network timeout' }));
      runCommand.withArgs(NPM_BIN_PATH).resolves(commandResult({ stdout: 'dashmate/4.1.0 linux-x64' }));

      await expect(upgradeNpm()).to.be.rejectedWith(PackageUpgradeError);

      expect(invocationsOf(NPM_PATH)).to.have.lengthOf(1);
    });

    it('should never ask sudo for anything on the npm path', async () => {
      readFile.withArgs(NPM_MANIFEST_PATH).resolves(JSON.stringify({ version: VERSION }));

      await upgradeNpm();

      expect(invocationsOf(SUDO)).to.have.lengthOf(0);
      expect(runInteractiveCommand).to.have.not.been.called();
    });

    it('should fall back to the npm beside the running node when the global prefix is unknown', async () => {
      pathExists.withArgs(NPM_PATH).resolves(false);
      pathExists.withArgs(path.join(path.dirname(NODE_EXEC_PATH), 'npm')).resolves(true);

      runCommand.withArgs(NPM_BIN_PATH).resolves(commandResult({ stdout: 'dashmate/4.2.0' }));

      const result = await upgradeNpm({ npmGlobalPrefix: null });

      expect(result.installedVersion).to.equal(VERSION);
    });

    it('should refuse to look npm up on PATH when it cannot be found beside the installation', async () => {
      pathExists.resolves(false);
      pathExists.withArgs(NPM_PATH).resolves(false);

      const error = await expect(upgradeNpm()).to.be.rejectedWith(PackageUpgradeError);

      expect(error.step).to.equal(UPGRADE_STEP.LOCATE_NPM);
      expect(error.packageState).to.equal(PACKAGE_STATE.UNCHANGED);
      expect(error.instructions).to.deep.equal([`npm install -g dashmate@${VERSION}`]);
    });
  });

  describe('input validation', () => {
    const malformedVersions = [
      'git+https://evil.example/dashmate.git',
      'file:/tmp/dashmate',
      'npm:dashmate@1.0.0',
      'https://evil.example/dashmate.tgz',
      'latest',
      '4.2.0 --registry=http://evil.example',
      '4.2.0;reboot',
      '4.2.0`id`',
      '$(id)',
      '../../../etc/passwd',
      'v4.2.0',
      '4.2',
      '4.2.0+build.1',
      '',
      null,
      undefined,
      42,
      { toString: () => '4.2.0' },
    ];

    malformedVersions.forEach((version) => {
      it(`should refuse ${JSON.stringify(version)} before running any subprocess`, async () => {
        const error = await expect(upgrade({ version })).to.be.rejectedWith(PackageUpgradeError);

        expect(error.step).to.equal(UPGRADE_STEP.VALIDATE_INPUT);
        expect(error.packageState).to.equal(PACKAGE_STATE.UNCHANGED);
        expect(runCommand).to.have.not.been.called();
        expect(runInteractiveCommand).to.have.not.been.called();
      });
    });

    it('should refuse a malformed version on the npm path, where it would become a package specifier', async () => {
      const error = await expect(upgrade({
        installMethod: INSTALL_METHODS.NPM,
        expectedDebVersion: null,
        version: 'git+https://evil.example/dashmate.git',
        rootAccess: null,
      })).to.be.rejectedWith(PackageUpgradeError);

      expect(error.step).to.equal(UPGRADE_STEP.VALIDATE_INPUT);
      expect(runCommand).to.have.not.been.called();
    });

    it('should refuse a deb upgrade with no exact Debian version to install', async () => {
      // Without it apt takes whatever the pool sorts highest, and prereleases share that pool
      const error = await expect(upgrade({ expectedDebVersion: null }))
        .to.be.rejectedWith(PackageUpgradeError);

      expect(error.step).to.equal(UPGRADE_STEP.VALIDATE_INPUT);
      expect(error.message).to.include('exact Debian version');
      expect(runCommand).to.have.not.been.called();
    });

    it('should refuse a malformed Debian version', async () => {
      const error = await expect(upgrade({ expectedDebVersion: '4.2.0-1; rm -rf /' }))
        .to.be.rejectedWith(PackageUpgradeError);

      expect(error.step).to.equal(UPGRADE_STEP.VALIDATE_INPUT);
      expect(runCommand).to.have.not.been.called();
    });

    it('should refuse a Debian version built from a different release than the one approved', async () => {
      const error = await expect(upgrade({ expectedDebVersion: '4.3.0-1' }))
        .to.be.rejectedWith(PackageUpgradeError);

      expect(error.step).to.equal(UPGRADE_STEP.VALIDATE_INPUT);
      expect(error.message).to.include('not built from dashmate 4.2.0');
    });

    it('should accept the Debian spelling of a prerelease as the same release', async () => {
      stubHealthyDebHost({ before: '4.1.0-1', after: '4.2.0~rc.3-1' });

      const result = await upgrade({ version: '4.2.0-rc.3', expectedDebVersion: '4.2.0~rc.3-1' });

      expect(result.installedVersion).to.equal('4.2.0~rc.3-1');
    });

    it('should refuse an apt sources path outside the sources directory', async () => {
      const error = await expect(upgrade({ sourcesFilePath: '/tmp/evil/dashmate.sources' }))
        .to.be.rejectedWith(PackageUpgradeError);

      expect(error.step).to.equal(UPGRADE_STEP.VALIDATE_INPUT);
      expect(error.message).to.include('/etc/apt/sources.list.d');
      expect(runCommand).to.have.not.been.called();
    });

    it('should refuse an apt sources path that traverses out of the sources directory', async () => {
      const error = await expect(upgrade({ sourcesFilePath: '/etc/apt/sources.list.d/../../shadow' }))
        .to.be.rejectedWith(PackageUpgradeError);

      expect(error.step).to.equal(UPGRADE_STEP.VALIDATE_INPUT);
      expect(runCommand).to.have.not.been.called();
    });

    it('should refuse an apt sources path with whitespace in it', async () => {
      const error = await expect(upgrade({ sourcesFilePath: '/etc/apt/sources.list.d/dash mate.sources' }))
        .to.be.rejectedWith(PackageUpgradeError);

      expect(error.step).to.equal(UPGRADE_STEP.VALIDATE_INPUT);
    });

    it('should refuse an install method it cannot drive', async () => {
      const error = await expect(upgrade({ installMethod: INSTALL_METHODS.TARBALL }))
        .to.be.rejectedWith(PackageUpgradeError);

      expect(error.step).to.equal(UPGRADE_STEP.VALIDATE_INPUT);
      expect(error.message).to.include('by hand');
      expect(runCommand).to.have.not.been.called();
    });

    [
      ['binPath', 'relative/dashmate'],
      ['binPath', '/usr/lib/../lib/dashmate/bin/dashmate'],
      ['npmGlobalPrefix', 'usr/local'],
      ['nodeExecPath', 'node'],
    ].forEach(([option, value]) => {
      it(`should refuse ${option} of "${value}", which it would execute`, async () => {
        const error = await expect(upgrade({ [option]: value })).to.be.rejectedWith(PackageUpgradeError);

        expect(error.step).to.equal(UPGRADE_STEP.VALIDATE_INPUT);
        expect(runCommand).to.have.not.been.called();
      });
    });

    [
      ['lockTimeoutSeconds', 1.5],
      ['lockTimeoutSeconds', -1],
      ['lockTimeoutSeconds', '600'],
      ['installTimeoutMs', -1],
      ['installTimeoutMs', Number.NaN],
    ].forEach(([option, value]) => {
      it(`should refuse ${option} of ${JSON.stringify(value)}`, async () => {
        const error = await expect(upgrade({ [option]: value })).to.be.rejectedWith(PackageUpgradeError);

        expect(error.step).to.equal(UPGRADE_STEP.VALIDATE_INPUT);
        expect(runCommand).to.have.not.been.called();
      });
    });

    [
      { elevation: ROOT_ACCESS.SUDO, euid: 1000, authenticated: true },
      {
        elevation: 'root', euid: 1000, authenticated: true, warnings: [],
      },
      {
        elevation: ROOT_ACCESS.SUDO, euid: 1000, authenticated: 'yes', warnings: [],
      },
      {
        elevation: ROOT_ACCESS.SUDO, euid: 1000, authenticated: true, warnings: [{ text: 'x' }],
      },
      'sudo',
    ].forEach((rootAccess, index) => {
      it(`should refuse a root access record it did not produce (${index})`, async () => {
        // A record without a warnings array used to throw a bare TypeError, whose packageState is
        // undefined - which a caller checking for "uncertain" reads as "safe to carry on"
        const error = await expect(upgrade({ rootAccess })).to.be.rejectedWith(PackageUpgradeError);

        expect(error.step).to.equal(UPGRADE_STEP.VALIDATE_INPUT);
        expect(error.packageState).to.equal(PACKAGE_STATE.UNCHANGED);
        expect(runCommand).to.have.not.been.called();
      });
    });
  });

  describe('failing loud', () => {
    it('should never reject with anything that has no package state', async () => {
      runCommand.withArgs(DPKG).throws(new TypeError('a seam that misbehaved'));

      const error = await expect(upgrade()).to.be.rejectedWith(PackageUpgradeError);

      expect(error.step).to.equal(UPGRADE_STEP.UNEXPECTED);
      expect(error.packageState).to.equal(PACKAGE_STATE.UNCHANGED);
      expect(error.message).to.include('a seam that misbehaved');
    });

    it('should call the package state uncertain when something unexpected happened after apt ran', async () => {
      runCommand.withArgs(DPKG, ['--audit']).resolves(commandResult());
      runCommand.withArgs(DPKG_QUERY)
        .onFirstCall().resolves(commandResult({ stdout: '4.1.0-1' }))
        .onSecondCall()
        .throws(new TypeError('a seam that misbehaved'));
      runCommand.withArgs(SUDO).resolves(commandResult());

      const error = await expect(upgrade()).to.be.rejectedWith(PackageUpgradeError);

      expect(error.step).to.equal(UPGRADE_STEP.UNEXPECTED);
      expect(error.packageState).to.equal(PACKAGE_STATE.UNCERTAIN);
      expect(error.instructions).to.include('sudo dpkg --configure -a');
    });

    it('should carry the elevation on the failure path, not only on success', async () => {
      stubFailingApt({ exitCode: 100, stderr: 'E: Sub-process returned an error code' });

      const error = await expect(upgrade()).to.be.rejectedWith(PackageUpgradeError);

      expect(error.elevation).to.equal(ROOT_ACCESS.SUDO);
    });

    it('should always produce a package state a caller can branch on', async () => {
      stubFailingApt({ exitCode: 100, stderr: 'E: Sub-process returned an error code' });

      const error = await expect(upgrade()).to.be.rejectedWith(PackageUpgradeError);

      expect(Object.values(PACKAGE_STATE)).to.include(error.packageState);
    });
  });

  describe('sudo', () => {
    /**
     * @param {Object} [overrides]
     * @return {Promise<Object>}
     */
    function preflight(overrides = {}) {
      return ensureRootAccess({
        euid: 1000,
        env: HOSTILE_ENV,
        instructions: ['sudo apt-get install --only-upgrade dashmate=4.2.0-1'],
        sourcesFilePath: '/etc/apt/sources.list.d/dashmate.sources',
        expectedDebVersion: DEB_VERSION,
        runCommand,
        runInteractiveCommand,
        pathExists,
        ...overrides,
      });
    }

    beforeEach(() => {
      pathExists.withArgs(SUDO).resolves(true);
    });

    it('should ask sudo about the exact command it will run, not about an unrelated one', async () => {
      runCommand.withArgs(SUDO, SUDO_PREFLIGHT_ARGV).resolves(commandResult());

      const rootAccess = await preflight();

      expect(rootAccess.elevation).to.equal(ROOT_ACCESS.SUDO);
      expect(invocationsOf(SUDO)[0].args).to.deep.equal(SUDO_PREFLIGHT_ARGV);
      expect(runInteractiveCommand).to.have.not.been.called();
    });

    it('should fall back to asking whether sudo works at all when there is no command to ask about', async () => {
      runCommand.withArgs(SUDO, ['-n', 'true']).resolves(commandResult());

      const rootAccess = await preflight({ sourcesFilePath: null, expectedDebVersion: null });

      expect(rootAccess.elevation).to.equal(ROOT_ACCESS.SUDO);
      expect(invocationsOf(SUDO)[0].args).to.deep.equal(['-n', 'true']);
    });

    it('should refuse to go further when root cannot be reached without a prompt', async () => {
      runCommand.withArgs(SUDO).resolves(commandResult({
        exitCode: 1,
        stderr: 'sudo: a password is required',
      }));

      const error = await expect(preflight()).to.be.rejectedWith(PackageUpgradeError);

      expect(error.step).to.equal(UPGRADE_STEP.ROOT_ACCESS);
      expect(error.packageState).to.equal(PACKAGE_STATE.UNCHANGED);
      expect(error.instructions).to.deep.equal(['sudo apt-get install --only-upgrade dashmate=4.2.0-1']);
      expect(error.message).to.include('sudo apt-get install --only-upgrade dashmate=4.2.0-1');
      // nothing may hang waiting for a password nobody can type
      expect(runInteractiveCommand).to.have.not.been.called();
    });

    it('should tell an operator sudoers refuses the command rather than blame their password', async () => {
      runCommand.withArgs(SUDO).resolves(commandResult({ exitCode: 1, stderr: SUDOERS_REFUSAL }));

      const error = await expect(preflight({ interactive: true, isTTY: true }))
        .to.be.rejectedWith(PackageUpgradeError);

      expect(error.message).to.include('sudoers policy');
      expect(error.message).to.not.include('would ask for a password');
      expect(error.instructions.join('\n')).to.include(`Add to sudoers: <user> ALL=(root) NOPASSWD: ${ENV} -i`);
      // authenticating cannot help, so it is never offered
      expect(runInteractiveCommand).to.have.not.been.called();
    });

    it('should refuse rather than guess when sudo fails for a reason it does not recognise', async () => {
      runCommand.withArgs(SUDO).resolves(commandResult({ exitCode: 1, stderr: 'sudo: unable to open audit system' }));

      const error = await expect(preflight({ interactive: true, isTTY: true }))
        .to.be.rejectedWith(PackageUpgradeError);

      expect(error.message).to.include('does not recognise');
      expect(runInteractiveCommand).to.have.not.been.called();
    });

    it('should authenticate on the terminal only when asked to and only with a terminal to ask on', async () => {
      runCommand.withArgs(SUDO, SUDO_PREFLIGHT_ARGV)
        .onFirstCall().resolves(commandResult({ exitCode: 1, stderr: 'sudo: a password is required' }))
        .onSecondCall()
        .resolves(commandResult());
      runInteractiveCommand.withArgs(SUDO, ['-v']).resolves(commandResult());

      const rootAccess = await preflight({ interactive: true, isTTY: true });

      expect(rootAccess.elevation).to.equal(ROOT_ACCESS.SUDO);
      expect(runInteractiveCommand).to.have.been.calledOnceWith(SUDO, ['-v']);
      // and the exact command is re-checked afterwards, still without a prompt
      expect(runCommand.secondCall.args[1]).to.deep.equal(SUDO_PREFLIGHT_ARGV);
    });

    it('should refuse to prompt when there is no terminal, even when asked to', async () => {
      runCommand.withArgs(SUDO).resolves(commandResult({ exitCode: 1, stderr: 'sudo: a password is required' }));

      const error = await expect(preflight({ interactive: true, isTTY: false }))
        .to.be.rejectedWith(PackageUpgradeError);

      expect(error.message).to.include('no terminal');
      expect(runInteractiveCommand).to.have.not.been.called();
    });

    it('should report a cancelled password prompt rather than carry on', async () => {
      runCommand.withArgs(SUDO).resolves(commandResult({ exitCode: 1, stderr: 'sudo: a password is required' }));
      runInteractiveCommand.withArgs(SUDO, ['-v']).resolves(commandResult({ exitCode: 1 }));

      const error = await expect(preflight({ interactive: true, isTTY: true }))
        .to.be.rejectedWith(PackageUpgradeError);

      expect(error.step).to.equal(UPGRADE_STEP.ROOT_ACCESS);
    });

    it('should not need sudo when it is already root', async () => {
      const rootAccess = await preflight({ euid: 0 });

      expect(rootAccess.elevation).to.equal(ROOT_ACCESS.ALREADY_ROOT);
      expect(rootAccess.warnings[0]).to.include('/root/.dashmate');
      expect(runCommand).to.have.not.been.called();
    });

    it('should say so when sudo is not installed at all', async () => {
      pathExists.withArgs(SUDO).resolves(false);

      const error = await expect(preflight()).to.be.rejectedWith(PackageUpgradeError);

      expect(error.step).to.equal(UPGRADE_STEP.ROOT_ACCESS);
      expect(runCommand).to.have.not.been.called();
    });

    it('should establish root without a prompt when the caller did not do it first', async () => {
      runCommand.withArgs(SUDO).resolves(commandResult({ exitCode: 1, stderr: 'sudo: a password is required' }));

      const error = await expect(upgrade({ rootAccess: null })).to.be.rejectedWith(PackageUpgradeError);

      expect(error.step).to.equal(UPGRADE_STEP.ROOT_ACCESS);
      expect(runInteractiveCommand).to.have.not.been.called();
      expect(invocationsOf(APT_GET)).to.have.lengthOf(0);
    });

    it('should fail loudly when the sudo timestamp expires between the pre-flight and apt', async () => {
      stubFailingApt({ exitCode: 1, stderr: 'sudo: a password is required' });

      const error = await expect(upgrade()).to.be.rejectedWith(PackageUpgradeError);

      expect(error.step).to.equal(UPGRADE_STEP.APT_INSTALL);
      expect(error.message).to.include('has expired');
      expect(error.packageState).to.equal(PACKAGE_STATE.INTACT);
    });

    it('should tell a command whether it is running as root before the command builds anything', () => {
      expect(isRunningAsRoot({ euid: 0 })).to.be.true();
      expect(isRunningAsRoot({ euid: 1000 })).to.be.false();
      expect(isRunningAsRoot({ euid: null })).to.be.false();
    });
  });

  describe('environment', () => {
    it('should hand apt an allowlisted environment rather than the operator\'s', async () => {
      stubHealthyDebHost();

      await upgrade();

      const [apt] = invocationsOf(SUDO);

      // env -i rebuilds the environment from nothing, so what sudoers keeps cannot leak in
      const assignments = apt.args.slice(apt.args.indexOf('-i') + 1, apt.args.indexOf(APT_GET));

      expect(assignments).to.deep.equal([
        'PATH=/usr/sbin:/usr/bin:/sbin:/bin',
        'HOME=/root',
        'LANG=C',
        'LC_ALL=C',
        'DEBIAN_FRONTEND=noninteractive',
      ]);
    });

    it('should never let APT_CONFIG reach apt, since it can disable signature verification', async () => {
      stubHealthyDebHost();

      await upgrade();

      invocations().forEach((invocation) => {
        expect(Object.keys(invocation.options.env)).to.not.include('APT_CONFIG');
        expect(invocation.args.join(' ')).to.not.include('APT_CONFIG');
      });
    });

    it('should never let NODE_OPTIONS, NODE_PATH or an LD_ variable reach a subprocess', async () => {
      stubHealthyDebHost();

      await upgrade();

      const forbidden = ['NODE_OPTIONS', 'NODE_PATH', 'LD_PRELOAD', 'LD_LIBRARY_PATH'];

      invocations().forEach((invocation) => {
        forbidden.forEach((name) => {
          expect(Object.keys(invocation.options.env)).to.not.include(name);
          expect(invocation.args.join(' ')).to.not.include(name);
        });
      });
    });

    it('should never let an inherited PATH decide what a subprocess resolves', async () => {
      stubHealthyDebHost();

      await upgrade();

      invocations().forEach((invocation) => {
        expect(invocation.options.env.PATH).to.not.include('/tmp/evil');
      });
    });

    it('should give npm the node that is running dashmate rather than one found on PATH', async () => {
      pathExists.withArgs(NPM_PATH).resolves(true);
      readFile.withArgs(NPM_MANIFEST_PATH).resolves(JSON.stringify({ version: VERSION }));

      await upgrade({
        installMethod: INSTALL_METHODS.NPM,
        expectedDebVersion: null,
        binPath: NPM_BIN_PATH,
        npmGlobalPrefix: NPM_PREFIX,
        rootAccess: null,
      });

      const [npm] = invocationsOf(NPM_PATH);

      expect(npm.options.env.PATH).to.equal(`${path.dirname(NODE_EXEC_PATH)}:/usr/sbin:/usr/bin:/sbin:/bin`);
      expect(npm.options.env.HOME).to.equal('/home/dash');
      expect(Object.keys(npm.options.env)).to.not.include('NODE_OPTIONS');
    });
  });

  describe('subprocess discipline', () => {
    it('should pass a subprocess seam nothing but an argv array and an allowlisted set of options', async () => {
      stubHealthyDebHost();

      await upgrade();

      expect(invocations()).to.not.be.empty();

      invocations().forEach(({ file, args, options }) => {
        expect(path.isAbsolute(file)).to.be.true();
        expect(args).to.be.an('array');

        args.forEach((arg) => {
          expect(arg).to.be.a('string');
        });

        // an allowlist, so a new option - `shell` above all - fails here rather than passing
        // because nobody thought to assert its absence
        expect(Object.keys(options).sort()).to.deep.equal(['env', 'timeout']);
      });
    });

    it('should name every executable by absolute path', async () => {
      stubHealthyDebHost();

      await upgrade();

      const executables = new Set(invocations().map(({ file }) => file));

      expect([...executables]).to.have.members([DPKG, DPKG_QUERY, SUDO]);
    });

    it('should not build any command through a shell', async () => {
      // The seams are stubbed in every other test, so the source itself is what pins this:
      // one exec() or one shell: true anywhere below would make every argv guarantee moot.
      const source = await fsPromises.readFile(MODULE_PATH, 'utf8');

      expect(source).to.not.match(/\bshell\b\s*:/);
      expect(source).to.not.match(/(?<!execFile)(?<!\.)\bexec\s*\(/);
      expect(source).to.not.match(/\bexecSync\b/);
      expect(source).to.not.match(/\bspawnSync\b/);
      expect(source).to.not.match(/\bchildProcess\.exec\b/);
    });

    it('should refuse an executable that is not named by absolute path', () => {
      expect(() => assertArgv('apt-get', ['install'])).to.throw(TypeError, 'absolute path');
      expect(() => assertArgv(null, [])).to.throw(TypeError);
    });

    it('should refuse an argument that is not a plain single-line string', () => {
      expect(() => assertArgv(APT_GET, ['install', 'dashmate\nrm -rf /'])).to.throw(TypeError);
      expect(() => assertArgv(APT_GET, ['install', `x${String.fromCharCode(0)}`])).to.throw(TypeError);
      expect(() => assertArgv(APT_GET, ['install', 42])).to.throw(TypeError);
      expect(() => assertArgv(APT_GET, 'install')).to.throw(TypeError);
      expect(() => assertArgv(APT_GET, ['install', '--only-upgrade'])).to.not.throw();
    });

    it('should record every invocation it made so a failure can name what ran', async () => {
      stubHealthyDebHost();

      const result = await upgrade();

      expect(result.commands.map((command) => command.step)).to.deep.equal([
        UPGRADE_STEP.DPKG_AUDIT,
        UPGRADE_STEP.QUERY_INSTALLED_VERSION,
        UPGRADE_STEP.APT_INSTALL,
        UPGRADE_STEP.QUERY_INSTALLED_VERSION,
      ]);
    });
  });

  describe('reported output', () => {
    it('should strip escape sequences a package manager put in its output', async () => {
      const escape = String.fromCharCode(27);

      stubFailingApt({
        exitCode: 100,
        stderr: `${escape}[2J${escape}[31mE: dashmate could not be fetched${escape}[0m`,
      });

      const error = await expect(upgrade()).to.be.rejectedWith(PackageUpgradeError);

      expect(error.output).to.equal('E: dashmate could not be fetched');
      expect(error.output).to.not.include(escape);
    });

    it('should strip a carriage return, which would let a later line overwrite an earlier one', async () => {
      // On a terminal "...is broken<CR>Everything is fine." prints as "Everything is fine."
      const carriageReturn = String.fromCharCode(13);

      stubFailingApt({
        exitCode: 100,
        stderr: `E: dashmate is half-installed and the node is broken${carriageReturn}Everything is fine.`,
      });

      const error = await expect(upgrade()).to.be.rejectedWith(PackageUpgradeError);

      expect(error.output).to.not.include(carriageReturn);
      expect(error.output).to.include('half-installed and the node is broken');
    });

    it('should keep the newlines that make output readable', async () => {
      stubFailingApt({ exitCode: 100, stderr: 'E: first line\nE: second line' });

      const error = await expect(upgrade()).to.be.rejectedWith(PackageUpgradeError);

      expect(error.output).to.equal('E: first line\nE: second line');
    });

    it('should bound how much package manager output it repeats', async () => {
      stubFailingApt({ exitCode: 100, stderr: 'x'.repeat(50000) });

      const error = await expect(upgrade()).to.be.rejectedWith(PackageUpgradeError);

      expect(error.output.length).to.be.below(5000);
      expect(error.output).to.include('truncated');
    });
  });
});
