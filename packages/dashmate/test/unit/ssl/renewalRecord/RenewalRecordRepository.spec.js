import fs from 'fs';
import path from 'path';
import RenewalRecordRepository from '../../../../src/ssl/renewalRecord/RenewalRecordRepository.js';
import HomeDir from '../../../../src/config/HomeDir.js';
import RenewalRecord from '../../../../src/ssl/renewalRecord/RenewalRecord.js';

describe('RenewalRecordRepository', () => {
  let homeDir;
  let repository;

  beforeEach(() => {
    homeDir = HomeDir.createTemp();
    repository = new RenewalRecordRepository(homeDir);
  });

  afterEach(() => homeDir.remove());

  /**
   * @return {RenewalRecord}
   */
  function record() {
    return RenewalRecord.fromObject({
      provider: 'letsencrypt',
      outcome: 'failed',
      code: 'PORT_80_UNREACHABLE',
      attemptedAt: new Date().toISOString(),
      consecutiveFailures: 1,
    });
  }

  /**
   * @return {string}
   */
  function lockPath() {
    return path.join(
      homeDir.joinPath('base', 'platform', 'gateway', 'ssl'),
      '.renewal-generation.lock',
    );
  }

  describe('the generation fence', () => {
    it('should hand out increasing generations', () => {
      expect(repository.claimGeneration('base')).to.equal(1);
      expect(repository.claimGeneration('base')).to.equal(2);
    });

    // The fence is only a guard if nothing can claim between reading it and
    // acting on it. Creating the file before taking the lock left exactly that
    // gap on first use: two processes both find it absent, both write zero, and
    // the one that loses claims a generation the other already took.
    it('should not create the fence before it is held', () => {
      const fencePath = path.join(
        homeDir.joinPath('base', 'platform', 'gateway', 'ssl'),
        '.renewal-generation',
      );

      let fenceExistedWhenLockTaken = null;
      const realOpenSync = fs.openSync;

      fs.openSync = (file, ...rest) => {
        if (String(file).endsWith('.lock') && fenceExistedWhenLockTaken === null) {
          fenceExistedWhenLockTaken = fs.existsSync(fencePath);
        }

        return realOpenSync(file, ...rest);
      };

      try {
        repository.claimGeneration('base');
      } finally {
        fs.openSync = realOpenSync;
      }

      expect(fenceExistedWhenLockTaken, 'the lock was taken').to.not.be.null();
      expect(fenceExistedWhenLockTaken, 'the fence was created under the lock').to.be.false();
      expect(fs.existsSync(fencePath), 'and it does get created').to.be.true();
    });

    // Without an owner, a holder whose lock was broken as stale still releases
    // on its way out - deleting whatever lock is there by then, which is the
    // next holder's. Two processes then believe they hold the same fence.
    //
    // Exercised through the release itself: the lock is taken over while the
    // operation is still running, so the original holder reaches its release
    // with someone else's lock in place.
    it('should not release a lock taken over while it was working', () => {
      const realRmSync = fs.rmSync;
      let takenOver = false;

      fs.rmSync = (target, ...rest) => {
        // Mid-operation: the holder is past acquiring and into its work. The
        // record write goes through a file descriptor, so the removal path is
        // the one place a real path is visible from outside.
        if (!takenOver && String(target).endsWith('renewal.json')) {
          takenOver = true;
          fs.writeFileSync(lockPath(), 'another-process');
        }

        return realRmSync(target, ...rest);
      };

      try {
        repository.remove('base');
      } finally {
        fs.rmSync = realRmSync;
      }

      expect(takenOver, 'the takeover happened').to.be.true();
      expect(fs.existsSync(lockPath()), "the new holder's lock survives").to.be.true();
      expect(fs.readFileSync(lockPath(), 'utf8')).to.equal('another-process');
    });

    // Holding the lock when the work started says nothing about holding it when
    // the write happens. A holder suspended past the stale threshold has its
    // lock reclaimed, and another process may already have written newer state;
    // resuming and overwriting that is the corruption the fence exists to stop.
    it('should refuse to write once its lock has been taken over', () => {
      repository.claimGeneration('base');

      const realReadFileSync = fs.readFileSync;
      const realWriteFileSync = fs.writeFileSync;
      let takenOver = false;

      // Staged while the operation is under way but before it mutates: the
      // generation is read first, and ownership is checked after that.
      fs.readFileSync = (file, ...rest) => {
        if (!takenOver && String(file).endsWith('.renewal-generation')) {
          takenOver = true;
          realWriteFileSync(lockPath(), 'another-process');
        }

        return realReadFileSync(file, ...rest);
      };

      let applied;

      try {
        applied = repository.remove('base', 1);
      } finally {
        fs.readFileSync = realReadFileSync;
      }

      expect(takenOver, 'the takeover happened').to.be.true();
      expect(applied, 'the superseded holder did not apply its change').to.be.false();
    });

    // Reclamation is by age, not by asking whether the holder still exists.
    // That question cannot be asked here: the helper holds this lock from
    // inside a container that bind-mounts the same home directory, so its pids
    // and the host CLI's come from different namespaces.
    it('should eventually reclaim a lock nobody released', () => {
      fs.mkdirSync(path.dirname(lockPath()), { recursive: true });
      fs.writeFileSync(lockPath(), 'a-holder-that-never-came-back');
      // Older than the stale threshold.
      const old = new Date(Date.now() - 60 * 1000);
      fs.utimesSync(lockPath(), old, old);

      expect(repository.claimGeneration('base')).to.equal(1);
    });

    // `claimGeneration` promises a number and its callers carry the result as
    // one. Returning a sentinel meant a chain held `false` as its generation and
    // every later write was fenced out by comparing against it - recording
    // nothing, quietly, which is what the record exists to prevent.
    it('should never hand back a generation that is not a number', () => {
      const generations = [
        repository.claimGeneration('base'),
        repository.claimGeneration('base'),
      ];

      generations.forEach((g) => expect(g).to.be.a('number'));
      expect(generations).to.deep.equal([1, 2]);
    });

    it('should record who holds it', () => {
      let held = null;
      const realRmSync = fs.rmSync;

      fs.rmSync = (target, ...rest) => {
        if (String(target).endsWith('.lock') && held === null) {
          held = fs.readFileSync(target, 'utf8');
        }

        return realRmSync(target, ...rest);
      };

      try {
        repository.claimGeneration('base');
      } finally {
        fs.rmSync = realRmSync;
      }

      expect(held, 'the lock names its holder').to.match(/^\d+\./);
    });
  });
});
