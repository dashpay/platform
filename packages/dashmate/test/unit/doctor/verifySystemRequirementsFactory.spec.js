import verifySystemRequirementsFactory from '../../../src/doctor/verifySystemRequirementsFactory.js';
import Problem from '../../../src/doctor/Problem.js';

describe('verifySystemRequirementsFactory', () => {
  let verifySystemRequirements;

  beforeEach(() => {
    verifySystemRequirements = verifySystemRequirementsFactory();
  });

  describe('CPU cores', () => {
    it('should return a problem if CPU cores are less than minimum for non evonode', () => {
      const systemInfo = {
        dockerSystemInfo: { NCPU: 1 },
      };

      const problems = verifySystemRequirements(systemInfo, false);

      expect(problems).to.have.lengthOf(1);
      expect(problems[0]).to.be.an.instanceOf(Problem);
      expect(problems[0].getDescription()).to.include('1 CPU cores detected');
    });

    it('should return a problem if CPU cores are less than minimum for evonode', () => {
      const systemInfo = {
        dockerSystemInfo: { NCPU: 2 },
      };

      const problems = verifySystemRequirements(systemInfo, true);

      expect(problems).to.have.lengthOf(1);
      expect(problems[0]).to.be.an.instanceOf(Problem);
      expect(problems[0].getDescription()).to.include('2 CPU cores detected');
    });

    it('should return a problem if CPU cores are less than minimum and docker info is not present', () => {
      const systemInfo = {
        cpu: { cores: 1 },
      };

      const problems = verifySystemRequirements(systemInfo, false);

      expect(problems).to.have.lengthOf(1);
      expect(problems[0]).to.be.an.instanceOf(Problem);
      expect(problems[0].getDescription()).to.include('1 CPU cores detected');
    });

    it('should not return anything if CPU cores information is not available', () => {
      const systemInfo = { };

      const problems = verifySystemRequirements(systemInfo, false);

      expect(problems).to.have.lengthOf(0);
    });
  });

  describe('CPU speed', () => {
    it('should return a problem if CPU speed is less than minimum', () => {
      const systemInfo = {
        cpu: { speed: 1.5 },
      };

      const problems = verifySystemRequirements(systemInfo, false);

      expect(problems).to.have.lengthOf(1);
      expect(problems[0]).to.be.an.instanceOf(Problem);
      expect(problems[0].getDescription()).to.include('1.5GHz CPU frequency detected');
    });

    it('should return a problem if CPU speed is not detected', () => {
      const systemInfo = {
        cpu: { speed: 0 },
      };

      const problems = verifySystemRequirements(systemInfo, false);

      expect(problems).to.have.lengthOf(0);
    });
  });

  describe('RAM', () => {
    it('should return a problem if RAM is less than minimum (from Docker info)', () => {
      const systemInfo = {
        dockerSystemInfo: { MemTotal: 2 * 1024 ** 3 },
      };

      const problems = verifySystemRequirements(systemInfo, false);

      expect(problems).to.have.lengthOf(1);
      expect(problems[0]).to.be.an.instanceOf(Problem);
      expect(problems[0].getDescription()).to.include('2.00GB RAM detected');
    });

    it('should return a problem if RAM is less than minimum (from memory)', () => {
      const systemInfo = {
        memory: { total: 2 * 1024 ** 3 },
      };

      const problems = verifySystemRequirements(systemInfo, false);

      expect(problems).to.have.lengthOf(1);
      expect(problems[0]).to.be.an.instanceOf(Problem);
      expect(problems[0].getDescription()).to.include('2.00GB RAM detected');
    });
  });

  describe('Swap', () => {
    it('should return a problem if swap space is less than recommended', () => {
      const systemInfo = {
        memory: { swaptotal: 1024 ** 3 },
      };

      const problems = verifySystemRequirements(systemInfo, false);

      expect(problems).to.have.lengthOf(1);
      expect(problems[0]).to.be.an.instanceOf(Problem);
      expect(problems[0].getDescription()).to.include('Swap space is 1.00GB');
    });
  });

  describe('Disk space', () => {
    it('should return a problem if disk space is less than minimum', () => {
      const systemInfo = {
        diskSpace: { free: 50 * 1024 ** 3 },
      };

      const problems = verifySystemRequirements(systemInfo, false);

      expect(problems).to.have.lengthOf(1);
      expect(problems[0]).to.be.an.instanceOf(Problem);
      expect(problems[0].getDescription()).to.include('50.00GB of available disk space detected');
    });
  });

  it('should not return any problems if all requirements are met', () => {
    const systemInfo = {
      dockerSystemInfo: { NCPU: 4, MemTotal: 8 * 1024 ** 3 },
      cpu: { cores: 4, speed: 3.0 },
      memory: { total: 8 * 1024 ** 3, swaptotal: 2 * 1024 ** 3 },
      diskSpace: { free: 500 * 1024 ** 3 },
    };

    const problems = verifySystemRequirements(systemInfo, false);

    expect(problems).to.have.lengthOf(0);
  });
});
