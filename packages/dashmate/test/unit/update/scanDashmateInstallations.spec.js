import { readFileSync } from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import scanDashmateInstallations, { ORIGIN } from '../../../src/update/scanDashmateInstallations.js';

const MODULE_PATH = fileURLToPath(new URL('../../../src/update/scanDashmateInstallations.js', import.meta.url));

const HOME_DIR = '/home/operator';

/**
 * A location that does not exist in any fixture, so the scan reports nothing
 * about the code running the test itself.
 */
const SELF_PATH = '/build/checkout/packages/dashmate/src/update/scanDashmateInstallations.js';

const dir = (options = {}) => ({ type: 'dir', ...options });
const file = (content = '') => ({ type: 'file', content });
const link = (target) => ({ type: 'link', target });
const manifest = (version) => file(JSON.stringify({ name: 'dashmate', version }));

function fsError(code, message) {
  const error = new Error(`${code}: ${message}`);
  error.code = code;

  return error;
}

/**
 * Fill in the directories implied by the fixture's paths, so a fixture only
 * has to name the files, symlinks and directories it cares about.
 *
 * @param {Object} entries
 * @return {Object}
 */
function buildTree(entries) {
  const tree = {};

  for (const location of Object.keys(entries)) {
    let current = path.posix.dirname(location);

    while (current !== '/' && current !== '.') {
      tree[current] = tree[current] ?? dir();
      current = path.posix.dirname(current);
    }
  }

  return { ...tree, ...entries };
}

/**
 * An in-memory POSIX filesystem, so no test touches the host.
 *
 * @param {Object} entries
 * @return {Object}
 */
function createFs(entries) {
  const tree = buildTree(entries);

  const resolve = (target, followFinal, depth = 0) => {
    if (depth > 10) {
      throw fsError('ELOOP', target);
    }

    const parts = target.split('/').filter((part) => part.length > 0);

    let current = '';

    for (let index = 0; index < parts.length; index += 1) {
      const parent = current === '' ? dir() : tree[current];

      // Traversing a directory needs execute permission on it.
      if (parent.readable === false) {
        throw fsError('EACCES', current);
      }

      const next = `${current}/${parts[index]}`;
      const node = tree[next];

      if (node === undefined) {
        throw fsError('ENOENT', next);
      }

      const isLast = index === parts.length - 1;

      if (node.type === 'link' && (!isLast || followFinal)) {
        const linkTarget = node.target.startsWith('/')
          ? node.target
          : path.posix.join(current === '' ? '/' : current, node.target);

        current = resolve(linkTarget, true, depth + 1);

        continue;
      }

      if (!isLast && node.type !== 'dir') {
        throw fsError('ENOTDIR', next);
      }

      current = next;
    }

    return current === '' ? '/' : current;
  };

  const nodeAt = (target, followFinal) => tree[resolve(target, followFinal)];

  const toStats = (node) => ({
    isFile: () => node.type === 'file',
    isDirectory: () => node.type === 'dir',
    isSymbolicLink: () => node.type === 'link',
  });

  return {
    lstat: async (target) => toStats(nodeAt(target, false)),
    stat: async (target) => toStats(nodeAt(target, true)),
    realpath: async (target) => resolve(target, true),
    readFile: async (target) => {
      const node = nodeAt(target, true);

      if (node.type !== 'file') {
        throw fsError('EISDIR', target);
      }

      return node.content;
    },
    readdir: async (target) => {
      const resolved = resolve(target, true);
      const node = tree[resolved];

      if (node.type !== 'dir') {
        throw fsError('ENOTDIR', target);
      }

      if (node.readable === false) {
        throw fsError('EACCES', target);
      }

      const prefix = `${resolved}/`;

      return Object.keys(tree)
        .filter((location) => location.startsWith(prefix)
          && !location.slice(prefix.length).includes('/'))
        .map((location) => location.slice(prefix.length));
    },
  };
}

function scan(entries, overrides = {}) {
  return scanDashmateInstallations({
    platform: 'linux',
    env: { PATH: '/usr/local/bin:/usr/bin' },
    fs: createFs(entries),
    homeDir: HOME_DIR,
    execPath: '/usr/local/bin/node',
    selfPath: SELF_PATH,
    cwd: '/',
    ...overrides,
  });
}

const NPM_ROOT = '/usr/local/lib/node_modules/dashmate';
const DEB_ROOT = '/usr/lib/dashmate';
const NVM_ROOT = `${HOME_DIR}/.nvm/versions/node/v20.11.0/lib/node_modules/dashmate`;

const npmInstall = (version = '4.1.0') => ({
  [`${NPM_ROOT}/package.json`]: manifest(version),
  [`${NPM_ROOT}/bin/run.js`]: file(),
  '/usr/local/bin/dashmate': link(`${NPM_ROOT}/bin/run.js`),
});

const debInstall = (version = '4.0.0') => ({
  [`${DEB_ROOT}/package.json`]: manifest(version),
  [`${DEB_ROOT}/bin/dashmate`]: file(),
  '/usr/bin/dashmate': link(`${DEB_ROOT}/bin/dashmate`),
});

const nvmInstall = (version = '4.1.0') => ({
  [`${NVM_ROOT}/package.json`]: manifest(version),
  [`${NVM_ROOT}/bin/run.js`]: file(),
  [`${HOME_DIR}/.nvm/versions/node/v20.11.0/bin/dashmate`]: link('../lib/node_modules/dashmate/bin/run.js'),
});

describe('scanDashmateInstallations', () => {
  describe('single installation', () => {
    it('should report one installation with its origin and version and no conflict', async () => {
      const result = await scan(npmInstall('4.1.0'));

      expect(result.supported).to.be.true();
      expect(result.hasConflict).to.be.false();
      expect(result.warnings).to.have.lengthOf(0);
      expect(result.installations).to.have.lengthOf(1);

      const [installation] = result.installations;

      expect(installation.root).to.equal(NPM_ROOT);
      expect(installation.origin).to.equal(ORIGIN.NPM_GLOBAL);
      expect(installation.version).to.equal('4.1.0');
      expect(installation.onPath).to.be.true();
      expect(installation.pathIndex).to.equal(0);
      expect(installation.active).to.be.true();
      expect(result.activePath).to.equal(NPM_ROOT);
    });

    it('should find an installation that is not on PATH through its known root', async () => {
      const result = await scan(debInstall('4.0.0'), { env: { PATH: '/usr/local/bin' } });

      expect(result.installations).to.have.lengthOf(1);

      const [installation] = result.installations;

      expect(installation.root).to.equal(DEB_ROOT);
      expect(installation.origin).to.equal(ORIGIN.DEB);
      expect(installation.onPath).to.be.false();
      expect(installation.pathIndex).to.be.null();
      expect(installation.active).to.be.false();
      expect(result.activePath).to.be.null();
      expect(result.hasConflict).to.be.false();
    });
  });

  describe('conflicting installations', () => {
    it('should report a deb and an npm installation as a conflict', async () => {
      const result = await scan({ ...debInstall('4.0.0'), ...npmInstall('4.1.0') });

      expect(result.hasConflict).to.be.true();
      expect(result.installations).to.have.lengthOf(2);
      expect(result.installations.map(({ origin }) => origin))
        .to.have.members([ORIGIN.DEB, ORIGIN.NPM_GLOBAL]);
      expect(result.installations.map(({ version }) => version))
        .to.have.members(['4.0.0', '4.1.0']);
    });

    it('should name the installation PATH resolves first as the one that actually runs', async () => {
      // The npm copy is earlier on PATH than the deb, so upgrading the deb
      // would leave the operator running unpatched code.
      const result = await scan({ ...debInstall('4.0.0'), ...npmInstall('4.1.0') });

      expect(result.activePath).to.equal(NPM_ROOT);
      expect(result.installations[0].active).to.be.true();
      expect(result.installations[0].root).to.equal(NPM_ROOT);
      expect(result.installations[1].active).to.be.false();
    });

    it('should report an nvm installation shadowing a deb installation', async () => {
      const result = await scan(
        { ...debInstall('4.0.0'), ...nvmInstall('4.1.0') },
        { env: { PATH: `${HOME_DIR}/.nvm/versions/node/v20.11.0/bin:/usr/bin` } },
      );

      expect(result.hasConflict).to.be.true();
      expect(result.activePath).to.equal(NVM_ROOT);
      expect(result.installations[0].origin).to.equal(ORIGIN.NVM);
      expect(result.installations[1].origin).to.equal(ORIGIN.DEB);
    });

    it('should mark the running installation even when PATH resolves a different one', async () => {
      // The deb binary is executing while the operator's shell resolves the npm
      // copy, which is exactly the case an upgrade would silently get wrong.
      const result = await scan(
        { ...debInstall('4.0.0'), ...npmInstall('4.1.0') },
        { selfPath: `${DEB_ROOT}/bin/dashmate` },
      );

      expect(result.runningPath).to.equal(DEB_ROOT);
      expect(result.activePath).to.equal(NPM_ROOT);

      const running = result.installations.find((installation) => installation.running);

      expect(running.root).to.equal(DEB_ROOT);
    });
  });

  describe('symlinks', () => {
    it('should report one installation when the same install is reachable through two symlinks', async () => {
      const result = await scan(
        {
          ...npmInstall('4.1.0'),
          [`${HOME_DIR}/bin/dashmate`]: link(`${NPM_ROOT}/bin/run.js`),
        },
        { env: { PATH: `${HOME_DIR}/bin:/usr/local/bin` } },
      );

      expect(result.hasConflict).to.be.false();
      expect(result.installations).to.have.lengthOf(1);

      const [installation] = result.installations;

      expect(installation.root).to.equal(NPM_ROOT);
      // Both PATH entries and the known-root probe all point at one install.
      expect(installation.entrypoints.map(({ path: entrypoint }) => entrypoint))
        .to.include.members([`${HOME_DIR}/bin/dashmate`, '/usr/local/bin/dashmate']);
      expect(installation.pathIndex).to.equal(0);
    });

    it('should ignore a dangling symlink and record it as a warning', async () => {
      const result = await scan({
        '/usr/local/bin/dashmate': link('/gone/dashmate/bin/run.js'),
      });

      expect(result.installations).to.have.lengthOf(0);
      expect(result.hasConflict).to.be.false();
      expect(result.warnings).to.have.lengthOf(1);
      expect(result.warnings[0].path).to.equal('/usr/local/bin/dashmate');
      expect(result.warnings[0].message).to.include('target no longer exists');
    });
  });

  describe('degraded scans', () => {
    it('should keep scanning and warn when a PATH directory cannot be read', async () => {
      const result = await scan(
        {
          ...npmInstall('4.1.0'),
          '/opt/private': dir({ readable: false }),
          '/opt/private/dashmate': file(),
        },
        { env: { PATH: '/opt/private:/usr/local/bin' } },
      );

      expect(result.installations).to.have.lengthOf(1);
      expect(result.installations[0].root).to.equal(NPM_ROOT);

      const warning = result.warnings
        .find(({ path: location }) => location === '/opt/private/dashmate');

      expect(warning).to.exist();
      expect(warning.code).to.equal('EACCES');
    });

    it('should warn and sweep only known roots when PATH is unset', async () => {
      const result = await scan(debInstall('4.0.0'), { env: {} });

      expect(result.supported).to.be.true();
      expect(result.warnings.map(({ code }) => code)).to.include('NO_PATH');
      expect(result.activePath).to.be.null();
      expect(result.installations).to.have.lengthOf(1);
      expect(result.installations[0].root).to.equal(DEB_ROOT);
    });

    it('should warn when PATH is an empty string', async () => {
      const result = await scan(npmInstall('4.1.0'), { env: { PATH: '' } });

      expect(result.warnings.map(({ code }) => code)).to.include('NO_PATH');
      // The npm install is still found through the known prefix.
      expect(result.installations).to.have.lengthOf(1);
      expect(result.installations[0].onPath).to.be.false();
    });

    it('should not throw when every filesystem call fails', async () => {
      const failingFs = {
        lstat: async () => { throw fsError('EIO', 'input/output error'); },
        stat: async () => { throw fsError('EIO', 'input/output error'); },
        realpath: async () => { throw fsError('EIO', 'input/output error'); },
        readFile: async () => { throw fsError('EIO', 'input/output error'); },
        readdir: async () => { throw fsError('EIO', 'input/output error'); },
      };

      const result = await scanDashmateInstallations({
        platform: 'linux',
        env: { PATH: '/usr/local/bin:/usr/bin' },
        fs: failingFs,
        homeDir: HOME_DIR,
        execPath: '/usr/local/bin/node',
        selfPath: SELF_PATH,
        cwd: '/',
      });

      expect(result.supported).to.be.true();
      expect(result.installations).to.have.lengthOf(0);
      expect(result.hasConflict).to.be.false();
      expect(result.warnings.length).to.be.greaterThan(0);
      expect(result.warnings.every(({ code }) => code === 'EIO')).to.be.true();
    });

    it('should not throw when an installation manifest is not valid JSON', async () => {
      const result = await scan({
        [`${NPM_ROOT}/package.json`]: file('{ this is not json'),
        [`${NPM_ROOT}/bin/run.js`]: file(),
        '/usr/local/bin/dashmate': link(`${NPM_ROOT}/bin/run.js`),
      });

      expect(result.installations).to.have.lengthOf(1);
      // Without a readable manifest there is no package root to attribute it to.
      expect(result.installations[0].root).to.be.null();
      expect(result.installations[0].version).to.be.null();
      expect(result.warnings.map(({ code }) => code)).to.include('INVALID_MANIFEST');
    });
  });

  describe('version manager shims', () => {
    it('should not count a version manager shim as a conflicting installation', async () => {
      const asdfRoot = `${HOME_DIR}/.asdf/installs/nodejs/20.11.0/lib/node_modules/dashmate`;

      const result = await scan(
        {
          [`${HOME_DIR}/.asdf/shims/dashmate`]: file('#!/usr/bin/env bash'),
          [`${asdfRoot}/package.json`]: manifest('4.1.0'),
          [`${asdfRoot}/bin/run.js`]: file(),
        },
        { env: { PATH: `${HOME_DIR}/.asdf/shims:/usr/bin` } },
      );

      expect(result.installations).to.have.lengthOf(2);
      expect(result.hasConflict).to.be.false();

      const shim = result.installations.find((installation) => installation.isShim);

      expect(shim).to.exist();
      expect(shim.origin).to.equal(ORIGIN.SHIM);
      expect(shim.active).to.be.true();

      const installation = result.installations.find(({ isShim }) => !isShim);

      expect(installation.root).to.equal(asdfRoot);
      expect(installation.origin).to.equal(ORIGIN.ASDF);
      expect(installation.version).to.equal('4.1.0');
    });
  });

  describe('platform support', () => {
    it('should report the scan as unsupported on Windows without touching the filesystem', async () => {
      const forbiddenFs = {
        lstat: async () => { throw new Error('filesystem must not be probed on Windows'); },
        stat: async () => { throw new Error('filesystem must not be probed on Windows'); },
        realpath: async () => { throw new Error('filesystem must not be probed on Windows'); },
        readFile: async () => { throw new Error('filesystem must not be probed on Windows'); },
        readdir: async () => { throw new Error('filesystem must not be probed on Windows'); },
      };

      const result = await scanDashmateInstallations({
        platform: 'win32',
        env: { PATH: 'C:\\Program Files\\nodejs' },
        fs: forbiddenFs,
        homeDir: 'C:\\Users\\operator',
        execPath: 'C:\\Program Files\\nodejs\\node.exe',
        selfPath: 'C:\\dashmate\\src\\update\\scanDashmateInstallations.js',
        cwd: 'C:\\',
      });

      // A caller must be able to tell "not checked" from "no conflict found",
      // and must not record an unsupported platform as a defect.
      expect(result.supported).to.be.false();
      expect(result.unsupportedReason).to.be.a('string');
      expect(result.installations).to.have.lengthOf(0);
      expect(result.hasConflict).to.be.false();
      expect(result.warnings).to.have.lengthOf(0);
    });

    it('should scan macOS the same way as Linux', async () => {
      const result = await scan(npmInstall('4.1.0'), { platform: 'darwin' });

      expect(result.supported).to.be.true();
      expect(result.installations).to.have.lengthOf(1);
    });
  });

  describe('result shape', () => {
    it('should return a result that survives a round trip through a state file', async () => {
      const result = await scan({ ...debInstall('4.0.0'), ...npmInstall('4.1.0') });

      expect(JSON.parse(JSON.stringify(result))).to.deep.equal(result);
    });

    it('should not execute anything it discovers', async () => {
      // Executing a binary found on PATH is the very risk this scan describes,
      // so the module must not reach for a subprocess API at all.
      const source = readFileSync(MODULE_PATH, 'utf8');

      expect(source).to.not.match(/child_process|execFile|spawn|execSync/);
    });
  });
});
