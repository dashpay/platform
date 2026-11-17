import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { debVersionFromSemver, debFileNameVersion } from '../../../scripts/deb-version.mjs';

const SCRIPT_PATH = fileURLToPath(new URL('../../../scripts/deb-version.mjs', import.meta.url));

/**
 * Independent port of dpkg's version comparison (`verrevcmp` in dpkg's version.c),
 * used as the oracle here so the expectations below describe what apt actually does
 * rather than whatever the mapping under test happens to produce.
 */
function order(char) {
  if (char === undefined) {
    return 0;
  }

  if (char >= '0' && char <= '9') {
    return 0;
  }

  if ((char >= 'a' && char <= 'z') || (char >= 'A' && char <= 'Z')) {
    return char.charCodeAt(0);
  }

  if (char === '~') {
    return -1;
  }

  return char.charCodeAt(0) + 256;
}

function isDigit(char) {
  return char !== undefined && char >= '0' && char <= '9';
}

function verrevcmp(left, right) {
  let i = 0;
  let j = 0;

  while (i < left.length || j < right.length) {
    let firstDiff = 0;

    while ((i < left.length && !isDigit(left[i])) || (j < right.length && !isDigit(right[j]))) {
      const leftOrder = order(left[i]);
      const rightOrder = order(right[j]);

      if (leftOrder !== rightOrder) {
        return leftOrder - rightOrder;
      }

      i += 1;
      j += 1;
    }

    while (left[i] === '0') {
      i += 1;
    }

    while (right[j] === '0') {
      j += 1;
    }

    while (isDigit(left[i]) && isDigit(right[j])) {
      if (firstDiff === 0) {
        firstDiff = left.charCodeAt(i) - right.charCodeAt(j);
      }

      i += 1;
      j += 1;
    }

    if (isDigit(left[i])) {
      return 1;
    }

    if (isDigit(right[j])) {
      return -1;
    }

    if (firstDiff !== 0) {
      return firstDiff;
    }
  }

  return 0;
}

function parseDebVersion(version) {
  const [, epoch = '0', rest] = /^(?:(\d+):)?(.*)$/.exec(version);
  const revisionAt = rest.lastIndexOf('-');

  return {
    epoch: Number(epoch),
    revision: revisionAt === -1 ? '' : rest.slice(revisionAt + 1),
    upstream: revisionAt === -1 ? rest : rest.slice(0, revisionAt),
  };
}

/**
 * @returns {number} negative when left sorts below right, 0 when equal, positive above
 */
function compareDebVersions(left, right) {
  const a = parseDebVersion(left);
  const b = parseDebVersion(right);

  if (a.epoch !== b.epoch) {
    return a.epoch - b.epoch;
  }

  const upstream = verrevcmp(a.upstream, b.upstream);

  return upstream === 0 ? verrevcmp(a.revision, b.revision) : upstream;
}

describe('deb-version.mjs', () => {
  describe('version comparison oracle', () => {
    it('should order versions the way dpkg does', () => {
      expect(compareDebVersions('4.1.0', '4.1.0')).to.equal(0);
      expect(compareDebVersions('4.1.0', '4.2.0')).to.be.below(0);
      expect(compareDebVersions('4.1.0', '4.1.10')).to.be.below(0);
      // `~` sorts below everything, including the end of the string
      expect(compareDebVersions('4.1.0~rc.1', '4.1.0')).to.be.below(0);
      // letters sort above digits, which is what makes a git sha unusable as a version part
      expect(compareDebVersions('4.1.0.a', '4.1.0.9')).to.be.above(0);
      // the Debian revision breaks ties on an identical upstream version
      expect(compareDebVersions('4.1.0-2', '4.1.0-1')).to.be.above(0);
      // an epoch outranks any upstream version
      expect(compareDebVersions('1:1.0.0', '99.0.0')).to.be.above(0);
    });
  });

  describe('versions produced by the legacy `<upstream>.<git sha>-1` scheme', () => {
    // Exactly what was published for the 4.1.0 series.
    const published = {
      'v4.1.0': '4.1.0.bfc80249b9-1',
      'v4.1.0-beta.2': '4.1.0.ae554fdd83-1',
      'v4.1.0-rc.1': '4.1.0.08152ea51e-1',
      'v4.1.0-rc.2': '4.1.0.3de436123d-1',
      'v4.1.0-rc.3': '4.1.0.61be67f7bf-1',
    };

    it('should make half of the real 4.1.0 releases look like downgrades to apt', () => {
      expect(compareDebVersions(published['v4.1.0-rc.1'], published['v4.1.0-beta.2'])).to.be.below(0);
      expect(compareDebVersions(published['v4.1.0-rc.2'], published['v4.1.0-rc.1'])).to.be.below(0);
      // and let the other half through, so the ordering is effectively arbitrary
      expect(compareDebVersions(published['v4.1.0-rc.3'], published['v4.1.0-rc.2'])).to.be.above(0);
      expect(compareDebVersions(published['v4.1.0'], published['v4.1.0-rc.3'])).to.be.above(0);
    });

    it('should never ship a rebuild of an already published version', () => {
      const hotfix = '4.1.0.284f02fabb-1';

      expect(compareDebVersions(hotfix, published['v4.1.0'])).to.be.below(0);
    });
  });

  describe('#debVersionFromSemver', () => {
    it('should give a stable release the first Debian revision', () => {
      expect(debVersionFromSemver('4.1.0')).to.equal('4.1.0-1');
      expect(debVersionFromSemver('v4.1.0')).to.equal('4.1.0-1');
    });

    it('should keep a prerelease below its final release', () => {
      expect(debVersionFromSemver('4.1.0-rc.3')).to.equal('4.1.0~rc.3-1');

      expect(compareDebVersions(
        debVersionFromSemver('4.1.0-rc.3'),
        debVersionFromSemver('4.1.0'),
      )).to.be.below(0);
    });

    it('should order every release of the 4.1.0 series upward', () => {
      const releases = ['4.1.0-beta.2', '4.1.0-rc.1', '4.1.0-rc.2', '4.1.0-rc.3', '4.1.0', '4.1.1'];

      releases.slice(1).forEach((release, index) => {
        const previous = debVersionFromSemver(releases[index]);
        const next = debVersionFromSemver(release);

        expect(compareDebVersions(next, previous)).to.be.above(
          0,
          `${next} must sort above ${previous}`,
        );
      });
    });

    // Releases go through dev, then beta, then rc, and `dev` sorts above `beta` by its
    // letters, so a plain `~dev` would make the first beta look like a downgrade. The 4.2.0
    // line went from dev.11 to beta.1; rc and the release complete the sequence.
    it('should order dev, beta and rc releases and the release upward, as 4.2.0 went', () => {
      const releases = [
        '4.2.0-dev.10', '4.2.0-dev.11', '4.2.0-beta.1', '4.2.0-beta.7', '4.2.0-rc.1', '4.2.0',
      ];

      releases.slice(1).forEach((release, index) => {
        const previous = debVersionFromSemver(releases[index]);
        const next = debVersionFromSemver(release);

        expect(compareDebVersions(next, previous)).to.be.above(
          0,
          `${next} must sort above ${previous}`,
        );
      });
    });

    it('should keep dev builds below every other prerelease of the same version', () => {
      expect(debVersionFromSemver('5.1.0-dev.11')).to.equal('5.1.0~~dev.11-1');

      ['5.1.0-alpha.1', '5.1.0-beta.1', '5.1.0-hotfix.1', '5.1.0-pr.1825.1', '5.1.0-rc.1'].forEach((prerelease) => {
        expect(compareDebVersions(
          debVersionFromSemver('5.1.0-dev.11'),
          debVersionFromSemver(prerelease),
        )).to.be.below(0, `dev must sort below ${prerelease}`);
      });
    });

    it('should treat only a `dev` label as a dev build', () => {
      expect(debVersionFromSemver('5.1.0-devnet.1')).to.equal('5.1.0~devnet.1-1');
    });

    // 5.0.x packages are built under the git sha scheme, the 5.0.0 release candidates among
    // them. The first 5.1 release must reach operators who installed one, without an epoch.
    it('should sort the first 5.1 dev build above every 5.0.0 git sha version', () => {
      ['5.0.0.8fccc88fc8-1', '5.0.0.ffffffffff-1', '5.0.0.9999999999-1'].forEach((legacy) => {
        expect(compareDebVersions(debVersionFromSemver('5.1.0-dev.1'), legacy)).to.be.above(0);
      });
    });

    // Inherent to any correct scheme, semver included, and the reason prereleases cannot
    // share a suite with stable: apt offers whatever sorts highest, so a stable node would
    // be walked onto a release candidate.
    it('should sort a prerelease of the next release above the current stable one', () => {
      expect(compareDebVersions(
        debVersionFromSemver('4.2.0-rc.1'),
        debVersionFromSemver('4.1.0'),
      )).to.be.above(0);
    });

    it('should let a rebuild of the same version overtake the published one', () => {
      expect(debVersionFromSemver('4.1.0', { revision: '2' })).to.equal('4.1.0-2');

      expect(compareDebVersions(
        debVersionFromSemver('4.1.0', { revision: '2' }),
        debVersionFromSemver('4.1.0'),
      )).to.be.above(0);
    });

    it('should overtake a legacy git sha version with an epoch', () => {
      const legacy = '4.1.0.bfc80249b9-1';

      // Without an epoch the legacy version wins, because `.bfc80249b9` extends `4.1.0`
      expect(compareDebVersions(debVersionFromSemver('4.1.0'), legacy)).to.be.below(0);

      expect(debVersionFromSemver('4.1.0', { epoch: '1' })).to.equal('1:4.1.0-1');
      expect(compareDebVersions(debVersionFromSemver('4.1.0', { epoch: '1' }), legacy)).to.be.above(0);
    });

    it('should treat an empty epoch as no epoch at all', () => {
      expect(debVersionFromSemver('4.1.0', { epoch: '' })).to.equal('4.1.0-1');
    });

    it('should refuse anything that is not a version it can translate', () => {
      expect(() => debVersionFromSemver('4.1')).to.throw('not a version');
      expect(() => debVersionFromSemver('4.1.0-rc.3; rm -rf /')).to.throw('not a version');
      // a `-` inside the prerelease would move where dpkg splits off the Debian revision
      expect(() => debVersionFromSemver('4.1.0-rc-3')).to.throw('not a version');
      // build metadata has no weight in semver ordering, so it cannot be given one here
      expect(() => debVersionFromSemver('4.1.0+build.5')).to.throw('not a version');
      expect(() => debVersionFromSemver('4.1.0', { revision: '1; id' })).to.throw('revision');
      expect(() => debVersionFromSemver('4.1.0', { epoch: 'x' })).to.throw('epoch');
    });

    it('should refuse leading zeroes, which dpkg would read as the same version', () => {
      expect(compareDebVersions('4.1.0~rc.01-1', '4.1.0~rc.1-1')).to.equal(0);
      expect(compareDebVersions('04.1.0-1', '4.1.0-1')).to.equal(0);
      expect(compareDebVersions('4.1.0-01', '4.1.0-1')).to.equal(0);
      expect(compareDebVersions('01:4.1.0-1', '1:4.1.0-1')).to.equal(0);

      expect(() => debVersionFromSemver('4.1.0-rc.01')).to.throw('not a version');
      expect(() => debVersionFromSemver('04.1.0')).to.throw('not a version');
      expect(() => debVersionFromSemver('4.01.0')).to.throw('not a version');
      // a hand-set revision or epoch of `01` would not overtake the `1` it was meant to
      expect(() => debVersionFromSemver('4.1.0', { revision: '01' })).to.throw('revision');
      expect(() => debVersionFromSemver('4.1.0', { epoch: '01' })).to.throw('epoch');
    });

    // An epoch of 0 is the same as none, so it would not overtake a git sha version, and a
    // revision of 0 sorts below the first build.
    it('should refuse a revision or an epoch of zero', () => {
      expect(compareDebVersions('0:4.1.0-1', '4.1.0-1')).to.equal(0);

      expect(() => debVersionFromSemver('4.1.0', { revision: '0' })).to.throw('revision');
      expect(() => debVersionFromSemver('4.1.0', { epoch: '0' })).to.throw('epoch');
    });
  });

  describe('#debFileNameVersion', () => {
    it('should leave a version that is already safe to publish alone', () => {
      expect(debFileNameVersion('4.1.0-1')).to.equal('4.1.0-1');
    });

    // GitHub rewrites `~` when a release asset is uploaded, so a name containing it would
    // not be the name the release ends up with.
    it('should drop the tilde a prerelease is published with', () => {
      expect(debFileNameVersion('4.1.0~rc.3-1')).to.equal('4.1.0.rc.3-1');
      expect(debFileNameVersion(debVersionFromSemver('4.1.0-beta.2'))).to.equal('4.1.0.beta.2-1');
      expect(debFileNameVersion(debVersionFromSemver('5.1.0-dev.11'))).to.equal('5.1.0.dev.11-1');
    });

    // Debian leaves the epoch out of file names, and `:` would not survive publishing either
    it('should drop the epoch', () => {
      expect(debFileNameVersion('1:4.1.0-1')).to.equal('4.1.0-1');
      expect(debFileNameVersion(debVersionFromSemver('4.1.0-rc.3', { epoch: '2' }))).to.equal('4.1.0.rc.3-1');
    });
  });

  describe('command line', () => {
    // Both variables are always given a value, so a rebuild that exports
    // DASHMATE_DEB_REVISION in the shell running the tests cannot reach the
    // script and change the version a test asserts.
    function runArgs(args, env) {
      return execFileSync(process.execPath, [SCRIPT_PATH, ...args], {
        encoding: 'utf8',
        stdio: 'pipe',
        env: {
          ...process.env,
          DASHMATE_DEB_REVISION: '1',
          DASHMATE_DEB_EPOCH: '',
          ...env,
        },
      }).trim();
    }

    function run(version, env) {
      return runArgs([version], env);
    }

    it('should take the revision and the epoch from the environment', () => {
      expect(run('v4.1.0', { DASHMATE_DEB_REVISION: '2' })).to.equal('4.1.0-2');
      expect(run('v4.1.0', { DASHMATE_DEB_EPOCH: '1' })).to.equal('1:4.1.0-1');
    });

    // The release workflow reads the epoch from a repository variable that is normally
    // unset, which exports it as set but empty. That has to mean no epoch at all.
    it('should ignore an epoch that is set to an empty value', () => {
      expect(run('v4.1.0', { DASHMATE_DEB_REVISION: '1', DASHMATE_DEB_EPOCH: '' })).to.equal('4.1.0-1');
      expect(run('v4.1.0-rc.3', { DASHMATE_DEB_REVISION: '1', DASHMATE_DEB_EPOCH: '' })).to.equal('4.1.0~rc.3-1');
    });

    it('should print the file name version with --file-name', () => {
      expect(runArgs(['--file-name', 'v5.1.0-dev.1'], { DASHMATE_DEB_EPOCH: '1' })).to.equal('5.1.0.dev.1-1');
    });

    // A mistyped command must fail rather than print a version for something else,
    // such as the control version when `--file-name` was put after the version.
    it('should refuse unexpected arguments', () => {
      expect(() => runArgs(['4.1.0', '1', '2'])).to.throw();
      expect(() => runArgs(['4.1.0', '--file-name'])).to.throw();
      expect(() => runArgs(['--file-name'])).to.throw();
      expect(() => runArgs([])).to.throw();
    });

    // pack_dashmate.sh relies on a failing exit status to stop the release build.
    it('should fail for a version or a revision it cannot translate', () => {
      expect(() => run('4.1')).to.throw();
      expect(() => run('4.1.0', { DASHMATE_DEB_REVISION: '01' })).to.throw();
    });
  });
});
