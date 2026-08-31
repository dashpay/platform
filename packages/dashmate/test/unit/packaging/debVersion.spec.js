import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { debVersionFromSemver, debFileNameVersion } from '../../../../../scripts/deb_version.js';

const SCRIPT_PATH = fileURLToPath(new URL('../../../../../scripts/deb_version.js', import.meta.url));

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

describe('deb_version.js', () => {
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

    it('should keep build metadata out of the upstream version', () => {
      // Semver gives build metadata no weight when ordering, so putting it in the upstream
      // version would let `4.1.0+build.5` outrank the plain `4.1.0` release
      expect(debVersionFromSemver('4.1.0+build.5')).to.equal('4.1.0-1+build.5');

      const repackaged = debVersionFromSemver('4.1.0+build.5');

      // it is still a distinct build, so it has to be installable over the plain one
      expect(compareDebVersions(repackaged, debVersionFromSemver('4.1.0'))).to.be.above(0);
      // but it must not overtake a revision bump or the next release
      expect(compareDebVersions(repackaged, debVersionFromSemver('4.1.0', { revision: '2' }))).to.be.below(0);
      expect(compareDebVersions(repackaged, debVersionFromSemver('4.1.1'))).to.be.below(0);
    });

    it('should refuse anything that is not a version it can translate', () => {
      expect(() => debVersionFromSemver('4.1')).to.throw('not a version');
      expect(() => debVersionFromSemver('4.1.0-rc.3; rm -rf /')).to.throw('not a version');
      // a `-` inside the prerelease would move where dpkg splits off the Debian revision
      expect(() => debVersionFromSemver('4.1.0-rc-3')).to.throw('not a version');
      expect(() => debVersionFromSemver('4.1.0', { revision: '1; id' })).to.throw('revision');
      expect(() => debVersionFromSemver('4.1.0', { epoch: 'x' })).to.throw('epoch');
    });

    it('should refuse leading zeroes, which dpkg would read as the same version', () => {
      expect(compareDebVersions('4.1.0~rc.01-1', '4.1.0~rc.1-1')).to.equal(0);
      expect(compareDebVersions('04.1.0-1', '4.1.0-1')).to.equal(0);

      expect(() => debVersionFromSemver('4.1.0-rc.01')).to.throw('not a version');
      expect(() => debVersionFromSemver('04.1.0')).to.throw('not a version');
      expect(() => debVersionFromSemver('4.01.0')).to.throw('not a version');
    });
  });

  describe('#debFileNameVersion', () => {
    it('should leave a version that is already safe to publish alone', () => {
      expect(debFileNameVersion('4.1.0-1')).to.equal('4.1.0-1');
    });

    // GitHub rewrites characters like `~` when a release asset is uploaded, which would
    // leave the published file name disagreeing with the index that points at it.
    it('should drop the tilde a prerelease is published with', () => {
      expect(debFileNameVersion('4.1.0~rc.3-1')).to.equal('4.1.0.rc.3-1');
      expect(debFileNameVersion(debVersionFromSemver('4.1.0-beta.2'))).to.equal('4.1.0.beta.2-1');
    });

    // Debian leaves the epoch out of file names, and `:` would not survive publishing either
    it('should drop the epoch', () => {
      expect(debFileNameVersion('1:4.1.0-1')).to.equal('4.1.0-1');
      expect(debFileNameVersion(debVersionFromSemver('4.1.0-rc.3', { epoch: '2' }))).to.equal('4.1.0.rc.3-1');
    });

    it('should not change the version apt installs', () => {
      // the file name is cosmetic; ordering comes from the control field, which keeps `~`
      expect(debVersionFromSemver('4.1.0-rc.3')).to.equal('4.1.0~rc.3-1');
      expect(compareDebVersions(
        debVersionFromSemver('4.1.0-rc.3'),
        debVersionFromSemver('4.1.0'),
      )).to.be.below(0);
    });
  });

  describe('command line', () => {
    // Both variables are always given a value, so a rebuild that exports
    // DASHMATE_DEB_REVISION in the shell running the tests cannot reach the
    // script and change the version a test asserts.
    function run(version, env) {
      return execFileSync(process.execPath, [SCRIPT_PATH, version], {
        encoding: 'utf8',
        env: {
          ...process.env,
          DASHMATE_DEB_REVISION: '1',
          DASHMATE_DEB_EPOCH: '',
          ...env,
        },
      }).trim();
    }

    it('should take the revision and the epoch from the environment', () => {
      expect(run('v4.1.0', { DASHMATE_DEB_REVISION: '2' })).to.equal('4.1.0-2');
      expect(run('v4.1.0', { DASHMATE_DEB_EPOCH: '1' })).to.equal('1:4.1.0-1');
    });

    // The release workflow declares both variables in one place so that the version gate
    // and the packaging job cannot read different values. That exports the epoch as set
    // but empty, which has to mean the same thing as not setting it at all.
    it('should ignore an epoch that is set to an empty value', () => {
      expect(run('v4.1.0', { DASHMATE_DEB_REVISION: '1', DASHMATE_DEB_EPOCH: '' })).to.equal('4.1.0-1');
      expect(run('v4.1.0-rc.3', { DASHMATE_DEB_REVISION: '1', DASHMATE_DEB_EPOCH: '' })).to.equal('4.1.0~rc.3-1');
    });
  });
});
