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
