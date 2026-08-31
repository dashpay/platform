import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { selectDebBaseline, releaseLine } from '../../../../../scripts/deb_release_baseline.js';

const SCRIPT_PATH = fileURLToPath(new URL('../../../../../scripts/deb_release_baseline.js', import.meta.url));

/**
 * A releases API entry, trimmed to the fields the baseline choice reads.
 *
 * `createdAt` is never used to order releases and is only set where a test needs the two
 * dates to disagree.
 */
function release({
  tag, publishedAt, createdAt = publishedAt, draft = false, assets = [],
}) {
  return {
    tag_name: tag,
    draft,
    created_at: createdAt,
    published_at: publishedAt,
    assets: assets.map((name) => ({ name })),
  };
}

function deb(version, arch = 'amd64') {
  return `dashmate_${version}_${arch}.deb`;
}

describe('deb_release_baseline.js', () => {
  describe('#releaseLine', () => {
    it('should group a prerelease with the release it leads up to', () => {
      expect(releaseLine('v4.1.0-rc.3')).to.equal('4.1.0'.split('.').slice(0, 2).join('.'));
      expect(releaseLine('v4.1.0-rc.3')).to.equal(releaseLine('v4.1.0'));
      expect(releaseLine('v4.1.7')).to.equal(releaseLine('v4.1.0'));
    });

    it('should keep separate minor lines apart', () => {
      expect(releaseLine('v4.2.0')).to.not.equal(releaseLine('v4.1.0'));
    });
  });

  describe('#selectDebBaseline', () => {
    it('should measure a release against the last package offered on its own line', () => {
      const releases = [
        release({ tag: 'v4.1.0', publishedAt: '2026-08-01T00:00:00Z', assets: [deb('4.1.0-1')] }),
        release({ tag: 'v4.1.1', publishedAt: '2026-08-10T00:00:00Z', assets: [deb('4.1.1-1')] }),
      ];

      expect(selectDebBaseline(releases, 'v4.1.2')).to.deep.equal({
        tag: 'v4.1.1',
        asset: deb('4.1.1-1'),
      });
    });

    // An operator on the 4.1 line was never offered 4.2, so measuring a 4.1 hotfix
    // against it would demand a version that outranks a release it does not supersede.
    it('should not measure a hotfix against a higher line', () => {
      const releases = [
        release({ tag: 'v4.1.1', publishedAt: '2026-08-01T00:00:00Z', assets: [deb('4.1.1-1')] }),
        release({ tag: 'v4.2.0', publishedAt: '2026-08-20T00:00:00Z', assets: [deb('4.2.0-1')] }),
      ];

      expect(selectDebBaseline(releases, 'v4.1.2').tag).to.equal('v4.1.1');
    });

    // The first release on a new line still has to outrank whatever apt last installed,
    // which is the newest package from the previous line.
    it('should fall back to the newest package of any line', () => {
      const releases = [
        release({ tag: 'v4.1.0', publishedAt: '2026-08-01T00:00:00Z', assets: [deb('4.1.0-1')] }),
        release({ tag: 'v4.1.1', publishedAt: '2026-08-10T00:00:00Z', assets: [deb('4.1.1-1')] }),
      ];

      expect(selectDebBaseline(releases, 'v4.2.0').tag).to.equal('v4.1.1');
    });

    // The package attached to the current tag is what an earlier run of this same release
    // already shipped, and it is exactly what apt measures a rebuild against. Ordering by
    // creation instead of publication loses that comparison whenever the release was
    // drafted before its own predecessor, and a same-version rebuild then passes the gate
    // while apt reports the package as already the newest version.
    it('should measure a rerun against the package that release already shipped', () => {
      const current = release({
        tag: 'v4.1.1',
        createdAt: '2026-08-01T00:00:00Z',
        publishedAt: '2026-08-20T00:00:00Z',
        assets: [deb('4.1.1-1')],
      });
      const predecessor = release({
        tag: 'v4.1.0',
        createdAt: '2026-08-05T00:00:00Z',
        publishedAt: '2026-08-06T00:00:00Z',
        assets: [deb('4.1.0-1')],
      });

      // The two orderings disagree, which is the whole point of the fixture: chosen by
      // creation the predecessor wins, chosen by publication the current release does.
      expect(current.created_at < predecessor.created_at).to.equal(true);
      expect(current.published_at > predecessor.published_at).to.equal(true);

      expect(selectDebBaseline([current, predecessor], 'v4.1.1').asset).to.equal(deb('4.1.1-1'));
    });

    // A draft has never been offered to anyone, so nothing can have installed it.
    it('should ignore drafts', () => {
      const releases = [
        release({ tag: 'v4.1.0', publishedAt: '2026-08-01T00:00:00Z', assets: [deb('4.1.0-1')] }),
        release({
          tag: 'v4.1.9', publishedAt: null, draft: true, assets: [deb('4.1.9-1')],
        }),
      ];

      expect(selectDebBaseline(releases, 'v4.1.1').tag).to.equal('v4.1.0');
    });

    // Prereleases go to the same channel as stable releases, so apt compares against them
    // like anything else.
    it('should measure against a prerelease', () => {
      const releases = [
        release({ tag: 'v4.1.0-rc.3', publishedAt: '2026-08-10T00:00:00Z', assets: [deb('4.1.0.rc.3-1')] }),
        release({ tag: 'v4.0.9', publishedAt: '2026-07-01T00:00:00Z', assets: [deb('4.0.9-1')] }),
      ];

      expect(selectDebBaseline(releases, 'v4.1.0').tag).to.equal('v4.1.0-rc.3');
    });

    it('should skip releases that shipped no package', () => {
      const releases = [
        release({ tag: 'v4.1.0', publishedAt: '2026-08-01T00:00:00Z', assets: [deb('4.1.0-1')] }),
        release({ tag: 'v4.1.1', publishedAt: '2026-08-10T00:00:00Z', assets: ['dashmate-v4.1.1-x64.pkg'] }),
      ];

      expect(selectDebBaseline(releases, 'v4.1.2').tag).to.equal('v4.1.0');
    });

    it('should have nothing to compare against when no release shipped a package', () => {
      expect(selectDebBaseline([], 'v4.1.0')).to.equal(null);
      expect(selectDebBaseline([
        release({ tag: 'v4.1.0', publishedAt: '2026-08-01T00:00:00Z' }),
      ], 'v4.1.1')).to.equal(null);
    });
  });

  describe('command line', () => {
    function run(currentTag, releases) {
      return execFileSync(process.execPath, [SCRIPT_PATH, currentTag], {
        encoding: 'utf8',
        input: JSON.stringify(releases),
      });
    }

    // The workflow pipes `gh api --paginate --slurp`, which emits one array per page.
    it('should read a paginated response and print the tag and package name', () => {
      const output = run('v4.1.2', [
        [release({ tag: 'v4.1.0', publishedAt: '2026-08-01T00:00:00Z', assets: [deb('4.1.0-1')] })],
        [release({ tag: 'v4.1.1', publishedAt: '2026-08-10T00:00:00Z', assets: [deb('4.1.1-1')] })],
      ]);

      expect(output).to.equal(`v4.1.1\t${deb('4.1.1-1')}\n`);
    });

    // The workflow reads an empty result as "nothing to compare against" and stops there,
    // so anything printed on the happy path would be taken for a package name.
    it('should print nothing when no release shipped a package', () => {
      expect(run('v4.1.0', [[]])).to.equal('');
    });
  });
});
